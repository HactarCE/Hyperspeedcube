//! 3x3x3x3 Rubik's cube.

use std::f32::consts::FRAC_PI_2;
use std::ops::{Add, Index, IndexMut, Mul, Neg};

use super::*;

/// Some pre-baked twists that can be applied to a 3x3x3x3 Rubik's cube.
pub mod twists {
    use super::*;

    // lazy_static! {
    //     /// Turn the right face clockwise 90 degrees.
    //     pub static ref R: Twist = Twist::new(Axis::X, Sign::Pos, TwistDirection::CW);
    //     /// Turn the left face clockwise 90 degrees.
    //     pub static ref L: Twist = Twist::new(Axis::X, Sign::Neg, TwistDirection::CW);
    //     /// Turn the top face clockwise 90 degrees.
    //     pub static ref U: Twist = Twist::new(Axis::Y, Sign::Pos, TwistDirection::CW);
    //     /// Turn the bottom face clockwise 90 degrees.
    //     pub static ref D: Twist = Twist::new(Axis::Y, Sign::Neg, TwistDirection::CW);
    //     /// Turn the front face clockwise 90 degrees.
    //     pub static ref F: Twist = Twist::new(Axis::Z, Sign::Pos, TwistDirection::CW);
    //     /// Turn the back face clockwise 90 degrees.
    //     pub static ref B: Twist = Twist::new(Axis::Z, Sign::Neg, TwistDirection::CW);

    //     /// Turn the middle layer down 90 degrees.
    //     pub static ref M: Twist = L.slice();
    //     /// Turn the equitorial layer to the right 90 degrees.
    //     pub static ref E: Twist = D.slice();
    //     /// Turn the standing layer clockwise 90 degrees.
    //     pub static ref S: Twist = F.slice();

    //     /// Turn the whole cube 90 degrees up.
    //     pub static ref X: Twist = R.whole_cube();
    //     /// Turn the whole cube 90 degrees to left.
    //     pub static ref Y: Twist = U.whole_cube();
    //     /// Turn the whole cube 90 degrees clockwise.
    //     pub static ref Z: Twist = F.whole_cube();
    // }
}

/// State of a 3x3x3x3 Rubik's cube.
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
pub struct Rubiks4D([[[Orientation; 3]; 3]; 3]);
impl PuzzleTrait for Rubiks4D {
    type Piece = Piece;
    type Sticker = Sticker;
    type Face = Face;
    type Twist = Twist;
    type Orientation = Orientation;

    const NDIM: usize = 4;
    const TYPE: PuzzleType = PuzzleType::Rubiks4D;

    fn get_piece(&self, pos: Piece) -> &Orientation {
        &self.0[pos.z_idx()][pos.y_idx()][pos.x_idx()]
    }
    fn get_piece_mut(&mut self, pos: Piece) -> &mut Orientation {
        &mut self.0[pos.z_idx()][pos.y_idx()][pos.x_idx()]
    }
    fn get_sticker(&self, pos: Sticker) -> Face {
        self.get_piece(pos.piece())[pos.axis()] * pos.sign()
    }

    fn radius(p: GeometryParams) -> f32 {
        p.face_scale * (1.0 + p.sticker_scale / 2.0)
    }
}

/// Piece location in a 3x3x3x3 Rubik's cube.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Piece(pub [Sign; 4]);
impl PieceTrait<Rubiks4D> for Piece {
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
            itertools::iproduct!(Sign::iter(), Sign::iter(), Sign::iter(), Sign::iter())
                .map(|(w, z, y, x)| Self([x, y, z, w]))
                .filter(|&p| p != Self::core()),
        )
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
        Self([Sign::Zero; 4])
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
    /// Returns the W coordinate of the piece.
    pub fn w(self) -> Sign {
        self[Axis::W]
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
    /// Returns the W coordinate of this piece, in the range 0..=2.
    fn w_idx(self) -> usize {
        (self.w().int() + 1) as usize
    }
}

/// Sticker location on a 3x3x3x3 Rubik's cube.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Sticker {
    piece: Piece,
    axis: Axis,
}
impl StickerTrait<Rubiks4D> for Sticker {
    const VERTEX_COUNT: u16 = 8;
    const SURFACE_INDICES: &'static [u16] = &[
        1, 2, 3, 2, 1, 0, // Z-
        7, 6, 5, 4, 5, 6, // Z+
        0, 1, 4, 5, 4, 1, // Y-
        6, 3, 2, 3, 6, 7, // Y+
        2, 4, 6, 4, 2, 0, // X-
        7, 5, 3, 1, 3, 5, // X+
    ];
    const OUTLINE_INDICES: &'static [u16] = &[
        0, 1, 1, 3, 3, 2, 2, 0, // Z-
        7, 6, 6, 4, 4, 5, 5, 7, // Z+
        0, 1, 1, 5, 5, 4, 4, 0, // Y-
        7, 6, 6, 2, 2, 3, 3, 7, // Y+
        0, 2, 2, 6, 6, 4, 4, 0, // X-
        7, 5, 5, 1, 1, 3, 3, 7, // X+
    ];

    fn piece(self) -> Piece {
        self.piece
    }
    fn face(self) -> Face {
        Face {
            axis: self.axis(),
            sign: self.sign(),
        }
    }
    fn center(self, p: GeometryParams) -> [f32; 4] {
        let (ax1, ax2, ax3) = self.face().parallel_axes();
        let mut ret = [0.0; 4];
        ret[self.axis() as usize] = 1.5 * self.sign().float();
        ret[ax1 as usize] = p.face_scale * self.piece()[ax1].float();
        ret[ax2 as usize] = p.face_scale * self.piece()[ax2].float();
        ret[ax3 as usize] = p.face_scale * self.piece()[ax3].float();
        ret
    }
    fn verts(self, p: GeometryParams) -> Vec<[f32; 4]> {
        let (ax1, ax2, ax3) = self.face().parallel_axes();
        let radius = p.face_scale * p.sticker_scale / 2.0;
        let center = self.center(p);
        itertools::iproduct!([-radius, radius], [-radius, radius], [-radius, radius])
            .map(|(v, u, t)| {
                let mut vert = center;
                vert[ax1 as usize] += t;
                vert[ax2 as usize] += u;
                vert[ax3 as usize] += v;
                vert
            })
            .collect()
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

/// Twist of a single face on a 3x3x3x3 Rubik's cube.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct Twist {
    //     face: Face,
//     direction: TwistDirection,
//     layers: [bool; 3],
}
impl TwistTrait<Rubiks4D> for Twist {
    fn rotation(self) -> Orientation {
        todo!()
    }
    fn rev(self) -> Self {
        todo!()
    }
    fn initial_pieces(self) -> Vec<Piece> {
        todo!()
    }
    fn matrix(self, portion: f32) -> cgmath::Matrix4<f32> {
        todo!()
    }
}
impl From<Sticker> for Twist {
    fn from(sticker: Sticker) -> Self {
        todo!()
    }
}
// impl Twist {
//     /// Returns a twist of the face with the given axis and sign in the given
//     /// direction.
//     pub fn new(axis: Axis, sign: Sign, direction: TwistDirection) -> Self {
//         Self {
//             face: Face { axis, sign },
//             direction,
//             layers: [true, false, false],
//         }
//     }
//     /// Make a fat (2-layer) move from this move.
//     pub fn fat(self) -> Self {
//         self.layers([true, true, false])
//     }
//     /// Make a slice move from this move.
//     pub fn slice(self) -> Self {
//         self.layers([false, true, false])
//     }
//     /// Make a whole cube rotation from this move.
//     pub fn whole_cube(self) -> Self {
//         self.layers([true, true, true])
//     }
//     /// Twist different layers.
//     pub fn layers(mut self, layers: [bool; 3]) -> Self {
//         self.layers = layers;
//         self
//     }
// }

/// 3-dimensional axis.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Axis {
    /// X axis (right).
    X = 0,
    /// Y axis (up).
    Y = 1,
    /// Z axis (towards the camera).
    Z = 2,
    /// W axis (towards the 4D eye).
    W = 3,
}
impl Axis {
    /// Returns the perpendicular axes from this one, in the order used for
    /// calculating sticker indices.
    pub fn sticker_order_perpendiculars(self) -> (Axis, Axis, Axis) {
        use Axis::*;
        // This ordering is necessary in order to maintain compatibility with
        // MC4D sticker indices.
        match self {
            X => (Y, Z, W),
            Y => (X, Z, W),
            Z => (X, Y, W),
            W => (X, Y, Z),
        }
    }

    /// Returns an iterator over all axes.
    pub fn iter() -> impl Iterator<Item = Axis> {
        [Axis::X, Axis::Y, Axis::Z, Axis::W].into_iter()
    }
}

/// Face of a 3D cube/cuboid.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Face {
    axis: Axis,
    sign: Sign,
}
impl FaceTrait<Rubiks4D> for Face {
    const COUNT: usize = 6;

    fn idx(self) -> usize {
        use Axis::*;
        use Sign::*;
        match (self.axis, self.sign) {
            (W, Neg) => 0, // In
            (Z, Neg) => 1, // Back
            (Y, Neg) => 2, // Down
            (X, Neg) => 3, // Left
            (X, Pos) => 4, // Right
            (Y, Pos) => 5, // Up
            (Z, Pos) => 6, // Front
            (W, Pos) => 7, // Out
            (_, Zero) => panic!("invalid face"),
        }
    }
    fn color(self) -> [f32; 3] {
        [
            crate::colors::PURPLE, // In
            crate::colors::YELLOW, // Back
            crate::colors::GREEN,  // Down
            crate::colors::ORANGE, // Left
            crate::colors::RED,    // Right
            crate::colors::BLUE,   // Up
            crate::colors::WHITE,  // Front
            crate::colors::PINK,   // Out
        ][self.idx()]
    }
    fn stickers(self) -> Box<dyn Iterator<Item = Sticker> + 'static> {
        let mut piece = self.center();
        let axis = self.axis;
        let (ax1, ax2, ax3) = self.axis.sticker_order_perpendiculars();
        Box::new(
            itertools::iproduct!(Sign::iter(), Sign::iter(), Sign::iter()).map(move |(v, u, t)| {
                piece[ax1] = t;
                piece[ax2] = u;
                piece[ax3] = v;
                Sticker { piece, axis }
            }),
        )
    }
    fn iter() -> Box<dyn Iterator<Item = Self>> {
        use Axis::*;
        use Sign::*;
        Box::new(
            [
                Self { axis: W, sign: Neg }, // In
                Self { axis: Z, sign: Neg }, // Back
                Self { axis: Y, sign: Neg }, // Down
                Self { axis: X, sign: Neg }, // Left
                Self { axis: X, sign: Pos }, // Right
                Self { axis: Y, sign: Pos }, // Up
                Self { axis: Z, sign: Pos }, // Front
                Self { axis: W, sign: Pos }, // Out
            ]
            .into_iter(),
        )
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
    pub fn parallel_axes(self) -> (Axis, Axis, Axis) {
        use Axis::*;
        let (ax1, ax2, ax3) = match self.axis {
            X => (Y, Z, W),
            Y => (X, W, Z),
            Z => (W, X, Y),
            W => (Z, Y, X),
        };
        match self.sign {
            Sign::Neg => (ax2, ax1, ax3),
            Sign::Zero => panic!("invalid face"),
            Sign::Pos => (ax1, ax2, ax3),
        }
    }
}

/// Orientation of a 4D cube (i.e. a single piece of a 4D cube/cuboid).
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Orientation([Face; 4]);
impl OrientationTrait<Rubiks4D> for Orientation {
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
            // Face color on the right cell.
            Face::new(Axis::X, Sign::Pos),
            // Face color on the top cell.
            Face::new(Axis::Y, Sign::Pos),
            // Face color on the front cell.
            Face::new(Axis::Z, Sign::Pos),
            // Face color on the outer cell.
            Face::new(Axis::W, Sign::Pos),
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
