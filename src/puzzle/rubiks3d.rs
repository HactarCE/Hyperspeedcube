//! 3x3x3 Rubik's cube.

use cgmath::{Deg, EuclideanSpace, Matrix4, Point3, Transform, Vector3, Zero};
use std::fmt;
use std::ops::{Add, Index, IndexMut, Mul, Neg};

use super::{traits::*, LayerMask, PieceType, PuzzleType, Sign, TwistDirection2D, TwistMetric};

/// Maximum extent of any single coordinate along the X, Y, or Z axes.
const PUZZLE_RADIUS: f32 = 1.5;

/// Some pre-baked twists that can be applied to a 3x3x3 Rubik's cube.
pub mod twists {
    use super::*;

    lazy_static! {
        /// Turn the right face clockwise 90 degrees.
        pub static ref R: Twist = Twist::from_face(Face::R, "CW").unwrap();
        /// Turn the left face clockwise 90 degrees.
        pub static ref L: Twist = Twist::from_face(Face::L, "CW").unwrap();
        /// Turn the top face clockwise 90 degrees.
        pub static ref U: Twist = Twist::from_face(Face::U, "CW").unwrap();
        /// Turn the bottom face clockwise 90 degrees.
        pub static ref D: Twist = Twist::from_face(Face::D, "CW").unwrap();
        /// Turn the front face clockwise 90 degrees.
        pub static ref F: Twist = Twist::from_face(Face::F, "CW").unwrap();
        /// Turn the back face clockwise 90 degrees.
        pub static ref B: Twist = Twist::from_face(Face::B, "CW").unwrap();

        /// Turn the middle layer down 90 degrees.
        pub static ref M: Twist = L.slice();
        /// Turn the equitorial layer to the right 90 degrees.
        pub static ref E: Twist = D.slice();
        /// Turn the standing layer clockwise 90 degrees.
        pub static ref S: Twist = F.slice();

        /// Turn the whole cube 90 degrees up.
        pub static ref X: Twist = R.whole_cube();
        /// Turn the whole cube 90 degrees to the left.
        pub static ref Y: Twist = U.whole_cube();
        /// Turn the whole cube 90 degrees clockwise.
        pub static ref Z: Twist = F.whole_cube();
    }
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
impl PuzzleState for Rubiks3D {
    type Piece = Piece;
    type Sticker = Sticker;
    type Face = Face;
    type Twist = Twist;
    type Orientation = Orientation;

    const NAME: &'static str = "Rubik's 3D";
    const TYPE: PuzzleType = PuzzleType::Rubiks3D;
    const NDIM: usize = 3;
    const LAYER_COUNT: usize = 3;

    const PIECE_TYPE_NAMES: &'static [&'static str] = &["center", "edge", "corner"];

    fn get_sticker_color(&self, pos: Sticker) -> Face {
        self[pos.piece()][pos.axis()] * pos.sign()
    }

    lazy_static_array_methods! {
        fn pieces() -> &'static [Piece] {
            itertools::iproduct!(Sign::iter(), Sign::iter(), Sign::iter())
                .map(|(z, y, x)| Piece([x, y, z]))
                .filter(|&p| p != Piece::core())
        }
        fn stickers() -> &'static [Sticker] {
            Rubiks3D::pieces().iter().copied().flat_map(Piece::stickers)
        }
    }
    fn faces() -> &'static [Face] {
        const RET: &[Face] = &[
            Face::new(Axis::Z, Sign::Neg), // Back
            Face::new(Axis::Y, Sign::Neg), // Down
            Face::new(Axis::X, Sign::Neg), // Left
            Face::new(Axis::X, Sign::Pos), // Right
            Face::new(Axis::Y, Sign::Pos), // Up
            Face::new(Axis::Z, Sign::Pos), // Front
        ];
        RET
    }

    lazy_static_generic_array_methods! {}

    fn face_symbols() -> &'static [&'static str] {
        &["B", "D", "L", "R", "U", "F"]
    }
    fn face_names() -> &'static [&'static str] {
        &["Back", "Down", "Left", "Right", "Up", "Front"]
    }

    fn twist_direction_symbols() -> &'static [&'static str] {
        &["", "'"]
    }
    fn twist_direction_names() -> &'static [&'static str] {
        &["CW", "CCW"]
    }
}
impl Rubiks3D {
    fn transform_point(point: Point3<f32>, p: StickerGeometryParams) -> Point3<f32> {
        let point = p.model_transform.transform_point(point);
        p.view_transform.transform_point(point / PUZZLE_RADIUS)
    }
}

/// Piece location in a 3x3x3 Rubik's cube.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Piece(pub [Sign; 3]);
impl FacetTrait for Piece {
    impl_facet_trait_id_methods!(Piece, Rubiks3D::pieces());

    fn projection_center(self, p: StickerGeometryParams) -> Option<Point3<f32>> {
        Some(Rubiks3D::transform_point(self.center_3d(p), p))
    }
}
impl PieceTrait<Rubiks3D> for Piece {
    fn piece_type(self) -> PieceType {
        PieceType {
            ty: Rubiks3D::TYPE,
            id: self.sticker_count() - 1,
        }
    }

    fn layer(self, face: Face) -> Option<usize> {
        match self[face.axis()] * face.sign() {
            Sign::Neg => Some(2),
            Sign::Zero => Some(1),
            Sign::Pos => Some(0),
        }
    }

    fn sticker_count(self) -> usize {
        self.x().abs() + self.y().abs() + self.z().abs()
    }
    fn stickers(self) -> Vec<Sticker> {
        Axis::iter()
            .filter(move |&axis| self[axis].is_nonzero())
            .map(move |axis| Sticker::new(self, axis))
            .collect()
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

    fn center_3d(self, p: StickerGeometryParams) -> Point3<f32> {
        let mut ret = Point3::origin();
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
impl FacetTrait for Sticker {
    impl_facet_trait_id_methods!(Sticker, Rubiks3D::stickers());

    fn projection_center(self, p: StickerGeometryParams) -> Option<Point3<f32>> {
        Some(Rubiks3D::transform_point(self.center_3d(p), p))
    }
}
impl StickerTrait<Rubiks3D> for Sticker {
    fn piece(self) -> Piece {
        self.piece
    }
    fn face(self) -> Face {
        let axis = self.axis;
        let sign = self.piece[axis];
        Face::new(axis, sign)
    }

    fn geometry(self, p: StickerGeometryParams) -> Option<StickerGeometry> {
        let (ax1, ax2) = self.face().parallel_axes();

        // Compute the center of the sticker.
        let center = self.center_3d(p);

        // Add a radius to the sticker along each axis.
        let sticker_radius = p.face_scale() * p.sticker_scale() / 2.0;
        let get_corner = |v, u| {
            let mut vert = center;
            vert[ax1 as usize] += u * sticker_radius;
            vert[ax2 as usize] += v * sticker_radius;
            Rubiks3D::transform_point(vert, p)
        };

        Some(StickerGeometry::new_double_quad([
            get_corner(-1.0, -1.0),
            get_corner(-1.0, 1.0),
            get_corner(1.0, -1.0),
            get_corner(1.0, 1.0),
        ]))
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

    fn center_3d(self, p: StickerGeometryParams) -> Point3<f32> {
        let mut ret = self.piece().center_3d(p);
        ret[self.axis() as usize] = 1.5 * self.sign().float();
        ret
    }
}

/// Face of a 3D cube/cuboid.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Face {
    axis: Axis,
    sign: Sign,
}
impl FacetTrait for Face {
    impl_facet_trait_id_methods!(Face, Rubiks3D::faces());

    fn projection_center(self, p: StickerGeometryParams) -> Option<Point3<f32>> {
        self.center_sticker().projection_center(p)
    }
}
impl FaceTrait<Rubiks3D> for Face {
    fn pieces(self, layer: usize) -> Vec<Piece> {
        let mut piece = self.center();
        for _ in 0..layer {
            piece = piece + self.opposite();
        }
        let (ax1, ax2) = self.axis.perpendiculars();
        itertools::iproduct!(Sign::iter(), Sign::iter())
            .map(move |(v, u)| {
                piece[ax1] = u;
                piece[ax2] = v;
                piece
            })
            .collect()
    }
    fn stickers(self) -> Vec<Sticker> {
        self.pieces(0)
            .into_iter()
            .map(move |piece| Sticker::new(piece, self.axis))
            .collect()
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
        assert!(sign.is_nonzero(), "invalid sign for face");
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
        Sticker::new(self.center(), self.axis)
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

/// Twist of a single face on a 3x3x3 Rubik's cube.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct Twist {
    /// Face to twist.
    pub face: Face,
    /// Direction to twist the face.
    pub direction: TwistDirection2D,
    /// Layer mask.
    pub layers: LayerMask,
}
impl fmt::Display for Twist {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.layers.0 {
            // Simple moves.
            0b001 => write!(f, "{}{}", self.face.symbol(), self.direction),

            // Wide moves.
            0b011 => write!(f, "{}w{}", self.face.symbol(), self.direction),

            // Slice moves.
            0b010 => match self.face.axis() {
                Axis::X => write!(f, "M{}", self.direction * -self.face.sign()),
                Axis::Y => write!(f, "E{}", self.direction * -self.face.sign()),
                Axis::Z => write!(f, "S{}", self.direction * self.face.sign()),
            },

            // Whole cube rotations.
            0b111 => match self.face.axis() {
                Axis::X => write!(f, "x{}", self.direction * self.face.sign()),
                Axis::Y => write!(f, "y{}", self.direction * self.face.sign()),
                Axis::Z => write!(f, "z{}", self.direction * self.face.sign()),
            },

            // Anything else has a bad layer mask.
            _ => write!(f, "<unknown twist>"),
        }
    }
}
impl TwistTrait<Rubiks3D> for Twist {
    fn from_face_with_layers(
        face: Face,
        direction: &str,
        layers: LayerMask,
    ) -> Result<Twist, &'static str> {
        let direction = match direction {
            "CW" => TwistDirection2D::CW,
            "CCW" => TwistDirection2D::CCW,
            _ => return Err("invalid direction"),
        };
        layers.validate::<Rubiks3D>()?;
        Ok(Self {
            face,
            direction,
            layers,
        })
    }
    fn from_face_recenter(face: Face) -> Result<Twist, &'static str> {
        match face {
            Face::R => Ok(*twists::Y),
            Face::L => Ok(twists::Y.rev()),
            Face::U => Ok(twists::X.rev()),
            Face::D => Ok(*twists::X),
            Face::F => Err("cannot recenter near face"),
            Face::B => Err("cannot recenter far face"),
            _ => Err("invalid face"),
        }
    }
    fn from_sticker(
        sticker: Sticker,
        direction: TwistDirection2D,
        layers: LayerMask,
    ) -> Result<Twist, &'static str> {
        layers.validate::<Rubiks3D>()?;
        Ok(Self {
            face: sticker.face(),
            direction,
            layers,
        })
    }

    fn model_transform(self, t: f32) -> cgmath::Matrix4<f32> {
        let mut axis = Vector3::zero();
        axis[self.face.axis() as usize] = self.face.sign().float();
        let angle = Deg(t * 90.0 * self.direction.sign().float());
        Matrix4::from_axis_angle(axis, angle)
    }

    fn rotation(self) -> Orientation {
        // Get the axes of the plane of rotation.
        let (ax1, ax2) = self.face.parallel_axes();
        let rot = Orientation::rot90(ax1, ax2);
        // Reverse orientation if counterclockwise.
        match self.direction {
            TwistDirection2D::CW => rot,
            TwistDirection2D::CCW => rot.rev(),
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

    fn can_combine(self, previous: Option<Self>, metric: TwistMetric) -> bool {
        if self.is_whole_puzzle_rotation() {
            match metric {
                TwistMetric::Qstm | TwistMetric::Ftm | TwistMetric::Stm => true,
                TwistMetric::Etm => false,
            }
        } else if let Some(prev) = previous {
            match metric {
                TwistMetric::Qstm => false,
                TwistMetric::Ftm => self.face == prev.face && self.layers == prev.layers,
                TwistMetric::Stm => self.face == prev.face,
                TwistMetric::Etm => false,
            }
        } else {
            false
        }
    }
    fn is_whole_puzzle_rotation(self) -> bool {
        self.layers == LayerMask::all::<Rubiks3D>()
    }
}
impl Twist {
    /// Make a wide (2-layer) move from this move.
    pub const fn wide(mut self) -> Self {
        self.layers = LayerMask(0b011);
        self
    }
    /// Make a slice move from this move.
    pub const fn slice(mut self) -> Self {
        self.layers = LayerMask(0b011);
        self
    }
    /// Make a whole cube rotation from this move.
    pub const fn whole_cube(mut self) -> Self {
        self.layers = LayerMask::all::<Rubiks3D>();
        self
    }
    /// Twist different layers.
    pub fn layers(mut self, layers: LayerMask) -> Result<Self, &'static str> {
        layers.validate::<Rubiks3D>()?;
        self.layers = layers;
        Ok(self)
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
