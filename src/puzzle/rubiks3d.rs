//! A 3x3x3 puzzle cube.

use std::f32::consts::FRAC_PI_2;
use std::ops::{Add, Index, IndexMut, Mul, Neg};

use super::*;

/// Some pre-baked twists that can be applied to a 3x3x3 puzzle cube.
pub mod twists {
    use super::*;

    lazy_static! {
        /// Turn the right face clockwise 90 degrees.
        pub static ref R: Twist = Twist::new(Axis::X, Sign::Pos, TwistDirection::CW);
        /// Turn the left face clockwise 90 degrees.
        pub static ref L: Twist = Twist::new(Axis::X, Sign::Neg, TwistDirection::CW);
        /// Turn the top face clockwise 90 degrees.
        pub static ref U: Twist = Twist::new(Axis::Y, Sign::Pos, TwistDirection::CW);
        /// Turn the bottom face clockwise 90 degrees.
        pub static ref D: Twist = Twist::new(Axis::Y, Sign::Neg, TwistDirection::CW);
        /// Turn the front face clockwise 90 degrees.
        pub static ref F: Twist = Twist::new(Axis::Z, Sign::Pos, TwistDirection::CW);
        /// Turn the back face clockwise 90 degrees.
        pub static ref B: Twist = Twist::new(Axis::Z, Sign::Neg, TwistDirection::CW);
    }
}

/// The state of 3x3x3 puzzle cube.
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
pub struct Rubiks3D([[[Orientation; 3]; 3]; 3]);
impl PuzzleTrait for Rubiks3D {
    type Piece = Piece;
    type Sticker = Sticker;
    type Face = Face;
    type Twist = Twist;
    type Orientation = Orientation;

    fn get_piece(&self, pos: Self::Piece) -> &Self::Orientation {
        &self.0[pos.z_idx()][pos.y_idx()][pos.x_idx()]
    }
    fn get_piece_mut(&mut self, pos: Self::Piece) -> &mut Self::Orientation {
        &mut self.0[pos.z_idx()][pos.y_idx()][pos.x_idx()]
    }
    fn get_sticker(&self, pos: Sticker) -> Face {
        self.get_piece(pos.piece())[pos.axis()] * pos.sign()
    }
}

/// A piece location in a 3x3x3 puzzle cube.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Piece(pub [Sign; 3]);
impl PieceTrait<Rubiks3D> for Piece {
    fn sticker_count(self) -> usize {
        self.x().abs() + self.y().abs() + self.z().abs()
    }
    fn stickers(self) -> Box<dyn Iterator<Item = Sticker> + 'static> {
        Box::new(
            Axis::iter()
                .filter(move |&&axis| self[axis].is_nonzero())
                .map(move |&axis| Sticker { piece: self, axis }),
        )
    }
    fn iter() -> Box<dyn Iterator<Item = Self>> {
        Box::new(
            Sign::iter()
                .flat_map(|&z| Sign::iter().map(move |&y| (y, z)))
                .flat_map(|(y, z)| Sign::iter().map(move |&x| Self([x, y, z])))
                .filter(|&p| p != Self::core()),
        )
    }
}
impl Index<Axis> for Piece {
    type Output = Sign;
    fn index(&self, axis: Axis) -> &Sign {
        &self.0[axis.int()]
    }
}
impl IndexMut<Axis> for Piece {
    fn index_mut(&mut self, axis: Axis) -> &mut Sign {
        &mut self.0[axis.int()]
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
    /// Returns the X coordinate of this piece, in the range 0..3.
    fn x_idx(self) -> usize {
        (self.x().int() + 1) as usize
    }
    /// Returns the Y coordinate of this piece, in the range 0..3.
    fn y_idx(self) -> usize {
        (self.y().int() + 1) as usize
    }
    /// Returns the Z coordinate of this piece, in the range 0..3.
    fn z_idx(self) -> usize {
        (self.z().int() + 1) as usize
    }
}

/// A sticker location on a 3x3x3 puzzle cube.
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
        Face {
            axis: self.axis(),
            sign: self.sign(),
        }
    }
    fn verts(self, size: f32) -> Vec<[f32; 4]> {
        let radius = size / 2.0;
        let mut center = [0.0; 4];
        center[self.axis().int()] = 1.5 * self.sign().float();
        let (ax1, ax2) = self.axis().perpendiculars();
        let mut ret = vec![center; 4];
        let mut i = 0;
        for &u in &[-radius, radius] {
            for &v in &[-radius, radius] {
                ret[i][ax1.int()] = u + self.piece()[ax1].float();
                ret[i][ax2.int()] = v + self.piece()[ax2].float();
                i += 1;
            }
        }
        ret
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
}

/// A twist of a single face on a 3x3x3 puzzle cube.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct Twist {
    face: Face,
    direction: TwistDirection,
}
impl TwistTrait<Rubiks3D> for Twist {
    fn rotation(self) -> Orientation {
        // Get the axes of the plane of rotation.
        let (ax1, ax2) = self.face.parallels();
        let mut rot = Orientation::rot90(ax1, ax2);
        // Reverse orientation if counterclockwise.
        if self.direction == TwistDirection::CCW {
            rot = rot.rev();
        }
        rot
    }
    fn rev(self) -> Self {
        Self {
            face: self.face,
            direction: self.direction.rev(),
        }
    }
    fn initial_pieces(self) -> Vec<Piece> {
        let (ax1, ax2) = self.face.parallels();
        let center = self.face.center();
        let mut edge = center;
        edge[ax1] = Sign::Neg;
        let mut corner = edge;
        corner[ax2] = Sign::Neg;
        vec![center, edge, corner]
    }
    fn matrix(self, portion: f32) -> cgmath::Matrix4<f32> {
        use cgmath::*;

        let (ax1, ax2) = self.face.parallels();
        let angle = portion * FRAC_PI_2 * self.direction.sign().float();

        let mut ret = Matrix4::identity();
        ret[ax1.int()][ax1.int()] = angle.cos();
        ret[ax1.int()][ax2.int()] = angle.sin();
        ret[ax2.int()][ax1.int()] = -angle.sin();
        ret[ax2.int()][ax2.int()] = angle.cos();

        ret
    }
}
impl From<Sticker> for Twist {
    fn from(sticker: Sticker) -> Self {
        Self::new(sticker.axis(), sticker.sign(), TwistDirection::CW)
    }
}
impl Twist {
    /// Returns a twist of the face with the given axis and sign in the given
    /// direction.
    pub fn new(axis: Axis, sign: Sign, direction: TwistDirection) -> Self {
        let face = Face { axis, sign };
        Self { face, direction }
    }
}

/// A 3-dimensional axis.
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
    /// Returns an integer index for this axis; X = 0, Y = 1, Z = 2.
    pub fn int(self) -> usize {
        match self {
            Self::X => 0,
            Self::Y => 1,
            Self::Z => 2,
        }
    }
    /// Returns the perpendicular axes from this one, using the left-hand rule.
    /// (The cross product of the returned axes is the opposite of the input.)
    /// This is more convenient for twisty puzzles, where clockwise rotations
    /// are the default.
    pub fn perpendiculars(self) -> (Axis, Axis) {
        match self {
            // X+ => rotate from Z+ to Y+.
            Axis::X => (Axis::Z, Axis::Y),
            // Y+ => rotate from X+ to Z+.
            Axis::Y => (Axis::X, Axis::Z),
            // Z+ => rotate from Y+ to X+.
            Axis::Z => (Axis::Y, Axis::X),
        }
    }
    /// Returns an iterator over all axes.
    pub fn iter() -> impl Iterator<Item = &'static Axis> {
        [Axis::X, Axis::Y, Axis::Z].iter()
    }
}

/// A face of a 3D cube/cuboid.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Face {
    axis: Axis,
    sign: Sign,
}
impl FaceTrait<Rubiks3D> for Face {
    const COUNT: usize = 6;

    fn idx(self) -> usize {
        match (self.axis, self.sign) {
            (Axis::X, Sign::Pos) => 0, // Right
            (Axis::X, Sign::Neg) => 1, // Left
            (Axis::Y, Sign::Pos) => 2, // Up
            (Axis::Y, Sign::Neg) => 3, // Down
            (Axis::Z, Sign::Pos) => 4, // Front
            (Axis::Z, Sign::Neg) => 5, // Back
            (_, Sign::Zero) => panic!("Invalid face"),
        }
    }
    fn stickers(self) -> Box<dyn Iterator<Item = Sticker> + 'static> {
        let mut piece = self.center();
        let axis = self.axis;
        let (ax1, ax2) = self.axis.perpendiculars();
        Box::new(Sign::iter().flat_map(move |&u| {
            piece[ax1] = u;
            Sign::iter().map(move |&v| {
                piece[ax2] = v;
                Sticker { piece, axis }
            })
        }))
    }
    fn iter() -> Box<dyn Iterator<Item = Self>> {
        Box::new(Axis::iter().flat_map(|&axis| Sign::iter().map(move |&sign| Self { axis, sign })))
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
    /// Returns the face on the given axis with the given sign. Panics if given
    /// Sign::Zero.
    pub fn new(axis: Axis, sign: Sign) -> Self {
        assert!(sign.is_nonzero(), "Invalid sign for face");
        Self { axis, sign }
    }
    /// Returns the axis perpendicular to this face.
    pub fn axis(self) -> Axis {
        self.axis
    }
    /// Returns the sign of this face along its perpendicular axis.
    pub fn sign(self) -> Sign {
        self.sign
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
    pub fn parallels(self) -> (Axis, Axis) {
        let (ax1, ax2) = self.axis.perpendiculars();
        match self.sign {
            Sign::Neg => (ax2, ax1),
            Sign::Zero => panic!("Invalid face"),
            Sign::Pos => (ax1, ax2),
        }
    }
}

/// An orientation of a 3D cube (i.e. a single piece of a 3D cube/cuboid).
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Orientation([Face; 3]);
impl OrientationTrait<Rubiks3D> for Orientation {
    fn rev(self) -> Self {
        let mut ret = Self::default();
        for &axis in Axis::iter() {
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
        &self.0[axis.int()]
    }
}
impl IndexMut<Axis> for Orientation {
    fn index_mut(&mut self, axis: Axis) -> &mut Face {
        &mut self.0[axis.int()]
    }
}
impl Mul<Orientation> for Orientation {
    type Output = Self;
    fn mul(self, rhs: Self) -> Self {
        let mut ret = Self::default();
        for &axis in Axis::iter() {
            ret[axis] = rhs[self[axis].axis] * self[axis].sign;
        }
        ret
    }
}
impl Mul<Piece> for Orientation {
    type Output = Piece;
    fn mul(self, rhs: Piece) -> Piece {
        let mut ret = Piece::core();
        for &axis in Axis::iter() {
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
        ret[axis2] = Face {
            axis: axis1,
            sign: Sign::Pos,
        };
        ret[axis1] = Face {
            axis: axis2,
            sign: Sign::Neg,
        };
        ret
    }
}
