//! 3x3x3 Rubik's cube.

use cgmath::{Deg, Matrix3, SquareMatrix, Vector3, Zero};
use std::fmt;
use std::ops::{Add, Index, IndexMut, Mul, Neg};

use super::*;
use crate::render::WireframeVertex;

/// Maximum extent of any single coordinate along the X, Y, or Z axes.
const PUZZLE_RADIUS: f32 = 1.5;

/// Some pre-baked twists that can be applied to a 3x3x3 Rubik's cube.
pub mod twists {
    use super::*;

    /// Turn the right face clockwise 90 degrees.
    pub const R: Twist = Twist::new(Face::R, TwistDirection::CW);
    /// Turn the left face clockwise 90 degrees.
    pub const L: Twist = Twist::new(Face::L, TwistDirection::CW);
    /// Turn the top face clockwise 90 degrees.
    pub const U: Twist = Twist::new(Face::U, TwistDirection::CW);
    /// Turn the bottom face clockwise 90 degrees.
    pub const D: Twist = Twist::new(Face::D, TwistDirection::CW);
    /// Turn the front face clockwise 90 degrees.
    pub const F: Twist = Twist::new(Face::F, TwistDirection::CW);
    /// Turn the back face clockwise 90 degrees.
    pub const B: Twist = Twist::new(Face::B, TwistDirection::CW);

    /// Turn the middle layer down 90 degrees.
    pub const M: Twist = L.slice();
    /// Turn the equitorial layer to the right 90 degrees.
    pub const E: Twist = D.slice();
    /// Turn the standing layer clockwise 90 degrees.
    pub const S: Twist = F.slice();

    /// Turn the whole cube 90 degrees up.
    pub const X: Twist = R.whole_cube();
    /// Turn the whole cube 90 degrees to left.
    pub const Y: Twist = U.whole_cube();
    /// Turn the whole cube 90 degrees clockwise.
    pub const Z: Twist = F.whole_cube();
}

/// State of a 3x3x3 Rubik's cube.
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
pub struct Rubiks3D([[[Orientation; 3]; 3]; 3]);
impl Index<Piece> for Rubiks3D {
    type Output = Orientation;

    fn index(&self, pos: Piece) -> &Self::Output {
        &self.0[pos.z_idx()][pos.y_idx()][pos.x_idx()]
    }
}
impl IndexMut<Piece> for Rubiks3D {
    fn index_mut(&mut self, pos: Piece) -> &mut Self::Output {
        &mut self.0[pos.z_idx()][pos.y_idx()][pos.x_idx()]
    }
}
impl PuzzleTrait for Rubiks3D {
    type Piece = Piece;
    type Sticker = Sticker;
    type Face = Face;
    type Twist = Twist;
    type Orientation = Orientation;

    const NDIM: usize = 3;
    const TYPE: PuzzleType = PuzzleType::Rubiks3D;

    fn get_sticker(&self, pos: Sticker) -> Face {
        self[pos.piece()][pos.axis()] * pos.sign()
    }
}
impl Rubiks3D {
    fn transform_point(point: Vector3<f32>, p: GeometryParams<Rubiks3D>) -> Vector3<f32> {
        p.transform * (point / PUZZLE_RADIUS)
    }
}

/// Piece location in a 3x3x3 Rubik's cube.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Piece(pub [Sign; 3]);
impl PieceTrait<Rubiks3D> for Piece {
    fn sticker_count(self) -> usize {
        self.x().abs() + self.y().abs() + self.z().abs()
    }
    fn stickers(self) -> Box<dyn Iterator<Item = Sticker> + 'static> {
        Box::new(
            Axis::iter()
                .filter(move |&axis| self[axis].is_nonzero())
                .map(move |axis| Sticker { piece: self, axis }),
        )
    }
    fn iter() -> Box<dyn Iterator<Item = Self>> {
        Box::new(
            itertools::iproduct!(Sign::iter(), Sign::iter(), Sign::iter())
                .map(|(z, y, x)| Self([x, y, z]))
                .filter(|&p| p != Self::core()),
        )
    }

    fn projection_center(self, p: GeometryParams<Rubiks3D>) -> Vector3<f32> {
        Rubiks3D::transform_point(self.center_3d(p), p)
    }
}
impl Index<Axis> for Piece {
    type Output = Sign;
    fn index(&self, axis: Axis) -> &Sign {
        &self.0[axis as usize]
    }
}
impl IndexMut<Axis> for Piece {
    fn index_mut(&mut self, axis: Axis) -> &mut Sign {
        &mut self.0[axis as usize]
    }
}
impl Add<Face> for Piece {
    type Output = Piece;
    fn add(mut self, rhs: Face) -> Self {
        self[rhs.axis] = self[rhs.axis] + rhs.sign;
        self
    }
}
impl Piece {
    /// Returns the piece at the center of the puzzle, which has no stickers.
    pub fn core() -> Self {
        Self([Sign::Zero; 3])
    }
    /// Returns the X coordinate of this piece.
    pub fn x(self) -> Sign {
        self[Axis::X]
    }
    /// Returns the Y coordinate of this piece.
    pub fn y(self) -> Sign {
        self[Axis::Y]
    }
    /// Returns the Z coordinate of the piece.
    pub fn z(self) -> Sign {
        self[Axis::Z]
    }
    /// Returns the X coordinate of this piece, in the range 0..=2.
    fn x_idx(self) -> usize {
        (self.x().int() + 1) as usize
    }
    /// Returns the Y coordinate of this piece, in the range 0..=2.
    fn y_idx(self) -> usize {
        (self.y().int() + 1) as usize
    }
    /// Returns the Z coordinate of this piece, in the range 0..=2.
    fn z_idx(self) -> usize {
        (self.z().int() + 1) as usize
    }

    fn center_3d(self, p: GeometryParams<Rubiks3D>) -> Vector3<f32> {
        let mut ret = Vector3::zero();
        for axis in Axis::iter() {
            ret[axis as usize] = p.face_scale() * self[axis].float();
        }
        ret
    }
}

/// Sticker location on a 3x3x3 Rubik's cube.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Sticker {
    piece: Piece,
    axis: Axis,
}
impl StickerTrait<Rubiks3D> for Sticker {
    const VERTEX_COUNT: u16 = 4;
    const SURFACE_INDICES: &'static [u16] = &[
        0, 1, 2, 3, 2, 1, // Outside face (counterclockwise from outside).
        1, 2, 3, 2, 1, 0, // Inside face (clockwise from outside).
    ];
    const OUTLINE_INDICES: &'static [u16] = &[0, 1, 1, 3, 3, 2, 2, 0];

    fn piece(self) -> Piece {
        self.piece
    }
    fn face(self) -> Face {
        Face::new(self.axis(), self.sign())
    }

    fn projection_center(self, p: GeometryParams<Rubiks3D>) -> Vector3<f32> {
        Rubiks3D::transform_point(self.center_3d(p), p)
    }
    fn verts(self, p: GeometryParams<Rubiks3D>) -> Option<Vec<WireframeVertex>> {
        let (ax1, ax2) = self.face().parallel_axes();
        let matrix = match p.anim {
            Some((twist, t)) if twist.affects_piece(self.piece) => twist.matrix(t),
            _ => Matrix3::identity(),
        };

        // Compute the center of the sticker.
        let center = self.center_3d(p);

        // Add a radius to the sticker along each axis.
        let sticker_radius = p.face_scale() * p.sticker_scale() / 2.0;
        let get_corner = |v, u| {
            let mut vert = center;
            vert[ax1 as usize] += u * sticker_radius;
            vert[ax2 as usize] += v * sticker_radius;
            Rubiks3D::transform_point(matrix * vert, p)
        };
        let corners = [
            get_corner(-1.0, -1.0),
            get_corner(-1.0, 1.0),
            get_corner(1.0, -1.0),
            get_corner(1.0, 1.0),
        ];
        Some(WireframeVertex::double_quad(corners, p.fill_color, p.wire_color).collect())
    }
}
impl Sticker {
    /// Returns the sticker on the given piece along the given axis. Panics if
    /// the given sticker does not exist.
    pub fn new(piece: Piece, axis: Axis) -> Self {
        assert!(
            piece[axis].is_nonzero(),
            "{:?} does not have sticker on {:?}",
            piece,
            axis
        );
        Self { piece, axis }
    }
    /// Returns the axis perpendicular to this sticker.
    pub fn axis(self) -> Axis {
        self.axis
    }
    /// Returns the sign of the normal of the sticker. (E.g. if the sticker is
    /// facing the positive direction along its axis, then this returns
    /// Sign::Pos; if it is facing the negative direction, this returns
    /// Sign::Neg).
    pub fn sign(self) -> Sign {
        self.piece()[self.axis()]
    }

    fn center_3d(self, p: GeometryParams<Rubiks3D>) -> Vector3<f32> {
        let mut ret = self.piece().center_3d(p);
        ret[self.axis() as usize] = 1.5 * self.sign().float();
        ret
    }
}

/// Twist of a single face on a 3x3x3 Rubik's cube.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct Twist {
    /// Face to twist.
    pub face: Face,
    /// Direction to twist the face.
    pub direction: TwistDirection,
    /// Layer mask.
    pub layers: [bool; 3],
}
impl fmt::Display for Twist {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.layers {
            // Simple moves and wide moves.
            [true, wide, false] => {
                let wide = if wide { "w" } else { "" };
                write!(f, "{}{}{}", self.face.symbol(), wide, self.direction)
            }

            // Slice moves.
            [false, true, false] => match self.face.axis() {
                Axis::X => write!(f, "M{}", self.direction.rev()),
                Axis::Y => write!(f, "E{}", self.direction.rev()),
                Axis::Z => write!(f, "S{}", self.direction),
            },

            // Whole cube rotations.
            [true, true, true] => match self.face.axis() {
                Axis::X => write!(f, "x{}", self.direction),
                Axis::Y => write!(f, "y{}", self.direction),
                Axis::Z => write!(f, "z{}", self.direction),
            },

            // Anything else has a bad layer mask.
            _ => write!(f, "<unknown twist>"),
        }
    }
}
impl TwistTrait<Rubiks3D> for Twist {
    fn rotation(self) -> Orientation {
        // Get the axes of the plane of rotation.
        let (ax1, ax2) = self.face.parallel_axes();
        let rot = Orientation::rot90(ax1, ax2);
        // Reverse orientation if counterclockwise.
        match self.direction {
            TwistDirection::CW => rot,
            TwistDirection::CCW => rot.rev(),
        }
    }
    fn rev(self) -> Self {
        Self {
            face: self.face,
            direction: self.direction.rev(),
            layers: self.layers,
        }
    }
    fn affects_piece(self, piece: Piece) -> bool {
        match piece[self.face.axis()] * self.face.sign() {
            Sign::Neg => self.layers[2],
            Sign::Zero => self.layers[1],
            Sign::Pos => self.layers[0],
        }
    }
}
impl From<Sticker> for Twist {
    fn from(sticker: Sticker) -> Self {
        Self::new(sticker.face(), TwistDirection::CW)
    }
}
impl Twist {
    /// Returns a twist of the face with the given axis and sign in the given
    /// direction.
    pub const fn new(face: Face, direction: TwistDirection) -> Self {
        Self {
            face,
            direction,
            layers: [true, false, false],
        }
    }
    /// Make a fat (2-layer) move from this move.
    pub const fn fat(self) -> Self {
        self.layers([true, true, false])
    }
    /// Make a slice move from this move.
    pub const fn slice(self) -> Self {
        self.layers([false, true, false])
    }
    /// Make a whole cube rotation from this move.
    pub const fn whole_cube(self) -> Self {
        self.layers([true, true, true])
    }
    /// Twist different layers.
    pub const fn layers(mut self, layers: [bool; 3]) -> Self {
        self.layers = layers;
        self
    }

    /// Returns a 4x4 rotation matrix for a portion of this twist, `t` ranges
    /// from 0.0 to 1.0. 0.0 gives the identity matrix; 1.0 gives the result of
    /// this twist, and intermediate values interpolate.
    fn matrix(self, t: f32) -> Matrix3<f32> {
        let mut axis = Vector3::zero();
        axis[self.face.axis() as usize] = self.face.sign().float();
        let angle = Deg(t * 90.0 * self.direction.sign().float());
        Matrix3::from_axis_angle(axis, angle)
    }
}

/// 3-dimensional axis.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Axis {
    /// X axis (right).
    X = 0,
    /// Y axis (up).
    Y = 1,
    /// Z axis (towards the camera).
    Z = 2,
}
impl Axis {
    /// Returns the perpendicular axes from this one, using the left-hand rule.
    /// (The cross product of the returned axes is the opposite of the input.)
    /// This is more convenient for twisty puzzles, where clockwise rotations
    /// are the default.
    pub fn perpendiculars(self) -> (Axis, Axis) {
        use Axis::*;
        match self {
            X => (Z, Y), // X+ => rotate from Z+ to Y+.
            Y => (X, Z), // Y+ => rotate from X+ to Z+.
            Z => (Y, X), // Z+ => rotate from Y+ to X+.
        }
    }

    /// Returns an iterator over all axes.
    pub fn iter() -> impl Iterator<Item = Axis> {
        [Axis::X, Axis::Y, Axis::Z].into_iter()
    }
}

/// Face of a 3D cube/cuboid.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Face {
    axis: Axis,
    sign: Sign,
}
impl FaceTrait<Rubiks3D> for Face {
    const COUNT: usize = 6;

    fn idx(self) -> usize {
        use Axis::*;
        use Sign::*;
        match (self.axis, self.sign) {
            (Z, Neg) => 0, // Back
            (Y, Neg) => 1, // Down
            (X, Neg) => 2, // Left
            (X, Pos) => 3, // Right
            (Y, Pos) => 4, // Up
            (Z, Pos) => 5, // Front
            (_, Zero) => panic!("invalid face"),
        }
    }
    fn symbol(self) -> char {
        b"BDLRUF"[self.idx()] as char
    }
    fn color(self) -> [f32; 3] {
        [
            crate::colors::BLUE,   // Back
            crate::colors::YELLOW, // Down
            crate::colors::ORANGE, // Left
            crate::colors::RED,    // Right
            crate::colors::WHITE,  // Up
            crate::colors::GREEN,  // Front
        ][self.idx()]
    }
    fn pieces(self) -> Box<dyn Iterator<Item = Piece> + 'static> {
        let mut piece = self.center();
        let (ax1, ax2) = self.axis.perpendiculars();
        Box::new(
            itertools::iproduct!(Sign::iter(), Sign::iter()).map(move |(v, u)| {
                piece[ax1] = u;
                piece[ax2] = v;
                piece
            }),
        )
    }
    fn stickers(self) -> Box<dyn Iterator<Item = Sticker> + 'static> {
        let axis = self.axis;
        Box::new(self.pieces().map(move |piece| Sticker::new(piece, axis)))
    }
    fn iter() -> Box<dyn Iterator<Item = Self>> {
        use Axis::*;
        use Sign::*;
        Box::new(
            [
                Self { axis: Z, sign: Neg }, // Back
                Self { axis: Y, sign: Neg }, // Down
                Self { axis: X, sign: Neg }, // Left
                Self { axis: X, sign: Pos }, // Right
                Self { axis: Y, sign: Pos }, // Up
                Self { axis: Z, sign: Pos }, // Front
            ]
            .into_iter(),
        )
    }

    fn projection_center(self, mut p: GeometryParams<Rubiks3D>) -> Vector3<f32> {
        p.anim = None;
        self.center_sticker().projection_center(p)
    }
}
impl Neg for Face {
    type Output = Face;
    fn neg(self) -> Self {
        Self {
            sign: -self.sign,
            ..self
        }
    }
}
impl Mul<Sign> for Face {
    type Output = Face;
    fn mul(self, rhs: Sign) -> Self {
        Self {
            sign: self.sign * rhs,
            ..self
        }
    }
}
impl Face {
    /// Right face.
    pub const R: Face = Face::new(Axis::X, Sign::Pos);
    /// Left face.
    pub const L: Face = Face::new(Axis::X, Sign::Neg);
    /// Top face.
    pub const U: Face = Face::new(Axis::Y, Sign::Pos);
    /// Bottom face.
    pub const D: Face = Face::new(Axis::Y, Sign::Neg);
    /// Front face.
    pub const F: Face = Face::new(Axis::Z, Sign::Pos);
    /// Back face.
    pub const B: Face = Face::new(Axis::Z, Sign::Neg);

    /// Returns the face on the given axis with the given sign. Panics if given
    /// Sign::Zero.
    pub const fn new(axis: Axis, sign: Sign) -> Self {
        // assert!(sign.is_nonzero(), "invalid sign for face"); // TODO: panicking in const functions is unstable
        Self { axis, sign }
    }
    /// Returns the axis perpendicular to this face.
    pub const fn axis(self) -> Axis {
        self.axis
    }
    /// Returns the sign of this face along its perpendicular axis.
    pub const fn sign(self) -> Sign {
        self.sign
    }
    /// Returns the opposite face.
    #[must_use]
    pub fn opposite(self) -> Self {
        Self {
            axis: self.axis,
            sign: -self.sign,
        }
    }
    /// Returns the piece at the center of this face.
    pub fn center(self) -> Piece {
        let mut ret = Piece::core();
        ret[self.axis] = self.sign;
        ret
    }
    /// Returns the sticker at the center of this face.
    pub fn center_sticker(self) -> Sticker {
        Sticker {
            piece: self.center(),
            axis: self.axis,
        }
    }
    /// Returns the axes parallel to this face (all except the perpendicular
    /// axis).
    pub fn parallel_axes(self) -> (Axis, Axis) {
        let (ax1, ax2) = self.axis.perpendiculars();
        match self.sign {
            Sign::Neg => (ax2, ax1),
            Sign::Zero => panic!("invalid face"),
            Sign::Pos => (ax1, ax2),
        }
    }
}

/// Orientation of a 3D cube (i.e. a single piece of a 3D cube/cuboid).
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Orientation([Face; 3]);
impl OrientationTrait<Rubiks3D> for Orientation {
    fn rev(self) -> Self {
        let mut ret = Self::default();
        for axis in Axis::iter() {
            ret[self[axis].axis] = Face::new(axis, self[axis].sign);
        }
        ret
    }
}
impl Default for Orientation {
    fn default() -> Self {
        Self([
            // Face color on the right side.
            Face::new(Axis::X, Sign::Pos),
            // Face color on the top side.
            Face::new(Axis::Y, Sign::Pos),
            // Face color on the front side.
            Face::new(Axis::Z, Sign::Pos),
        ])
    }
}
impl Index<Axis> for Orientation {
    type Output = Face;
    fn index(&self, axis: Axis) -> &Face {
        &self.0[axis as usize]
    }
}
impl IndexMut<Axis> for Orientation {
    fn index_mut(&mut self, axis: Axis) -> &mut Face {
        &mut self.0[axis as usize]
    }
}
impl Mul<Orientation> for Orientation {
    type Output = Self;
    fn mul(self, rhs: Self) -> Self {
        let mut ret = Self::default();
        for axis in Axis::iter() {
            ret[axis] = rhs[self[axis].axis] * self[axis].sign;
        }
        ret
    }
}
impl Mul<Piece> for Orientation {
    type Output = Piece;
    fn mul(self, rhs: Piece) -> Piece {
        let mut ret = Piece::core();
        for axis in Axis::iter() {
            ret[axis] = rhs[self[axis].axis] * self[axis].sign;
        }
        ret
    }
}
impl Orientation {
    /// Returns an orientation representing a 90-degree rotation from one axis
    /// to another.
    #[must_use]
    pub fn rot90(axis1: Axis, axis2: Axis) -> Self {
        let mut ret = Self::default();
        ret[axis2] = Face::new(axis1, Sign::Pos);
        ret[axis1] = Face::new(axis2, Sign::Neg);
        ret
    }
}
