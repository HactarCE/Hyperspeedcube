use std::ops::{Add, Index, IndexMut, Mul, Neg};

use super::common::*;

pub mod faces {
    use super::*;

    lazy_static! {
        pub static ref R: Sticker = Face::new(Axis::X, Sign::Pos).center_sticker();
        pub static ref L: Sticker = Face::new(Axis::X, Sign::Neg).center_sticker();
        pub static ref U: Sticker = Face::new(Axis::Y, Sign::Pos).center_sticker();
        pub static ref D: Sticker = Face::new(Axis::Y, Sign::Neg).center_sticker();
        pub static ref F: Sticker = Face::new(Axis::Z, Sign::Pos).center_sticker();
        pub static ref B: Sticker = Face::new(Axis::Z, Sign::Neg).center_sticker();
    }
}

/// A 3x3x3 Rubik's cube.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Puzzle([[[Orientation; 3]; 3]; 3]);
impl Index<Piece> for Puzzle {
    type Output = Orientation;
    fn index(&self, pos: Piece) -> &Orientation {
        &self.0[pos.z_idx()][pos.y_idx()][pos.x_idx()]
    }
}
impl IndexMut<Piece> for Puzzle {
    fn index_mut(&mut self, pos: Piece) -> &mut Orientation {
        &mut self.0[pos.z_idx()][pos.y_idx()][pos.x_idx()]
    }
}
impl Puzzle {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn get_sticker(&self, pos: Sticker) -> Face {
        self[pos.piece()][pos.axis()] * pos.sign()
    }
    pub fn swap(&mut self, pos1: Piece, pos2: Piece, rot: Orientation) {
        let tmp = self[pos1];
        self[pos1] = rot * self[pos2];
        self[pos2] = rot.rev() * tmp;
    }
    pub fn cycle(&mut self, start: Piece, rot: Orientation) {
        let rot = rot.rev();
        let mut prev = start;
        loop {
            let current = rot * prev;
            if current == start {
                break;
            }
            self.swap(current, prev, rot);
            prev = current;
        }
    }
    pub fn twist(&mut self, pos: Sticker, direction: TwistDirection) {
        // Cannot rotate around core.
        if pos.piece().sticker_count() == 0 {
            panic!("Cannot rotate around core");
        }
        // Get face.
        let face = pos.face();
        // Get perpendicular axes.
        let (ax1, ax2) = face.perpendiculars();
        let mut rot = Orientation::rot90(ax1, ax2);
        // Reverse rotation if rotation is CCW.
        if direction == TwistDirection::CCW {
            rot = rot.rev();
        }
        // Cycle edges.
        let edge_start = face.center() + Face::new(ax1, Sign::Pos);
        self.cycle(edge_start, rot);
        // Cycle corners.
        let corner_start = face.center() + Face::new(ax1, Sign::Pos) + Face::new(ax2, Sign::Pos);
        self.cycle(corner_start, rot);
    }
}

/// Unique identifier of a piece of the puzzle, or the location of a piece.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Piece(pub [Sign; 3]);
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
    pub const fn core() -> Self {
        Self([Sign::Zero; 3])
    }
    pub fn x(self) -> Sign {
        self[Axis::X]
    }
    pub fn y(self) -> Sign {
        self[Axis::Y]
    }
    pub fn z(self) -> Sign {
        self[Axis::Z]
    }
    fn x_idx(self) -> usize {
        (self.x().int() + 1) as usize
    }
    fn y_idx(self) -> usize {
        (self.y().int() + 1) as usize
    }
    fn z_idx(self) -> usize {
        (self.z().int() + 1) as usize
    }
    pub fn sticker_count(self) -> usize {
        self.x().abs() + self.y().abs() + self.z().abs()
    }
    pub fn stickers(self) -> impl Iterator<Item = Sticker> + 'static {
        Axis::iter()
            .filter(move |&&axis| self[axis].is_nonzero())
            .map(move |&axis| Sticker { piece: self, axis })
    }
    pub fn iter() -> impl Iterator<Item = Self> {
        Sign::iter()
            .flat_map(|&z| Sign::iter().map(move |&y| (y, z)))
            .flat_map(|(y, z)| Sign::iter().map(move |&x| Self([x, y, z])))
            .filter(|&p| p != Self::core())
    }
}

/// Unique identifier of a sticker on the puzzle, or the location of a sticker.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Sticker {
    pub piece: Piece,
    pub axis: Axis,
}
impl Sticker {
    pub fn new(piece: Piece, axis: Axis) -> Self {
        if piece[axis].is_zero() {
            panic!("{:?} does not have sticker on {:?}", piece, axis);
        }
        Self { piece, axis }
    }
    pub fn piece(self) -> Piece {
        self.piece
    }
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
    pub fn face(self) -> Face {
        Face {
            axis: self.axis(),
            sign: self.sign(),
        }
    }
    pub fn verts(self) -> [[f32; 3]; 4] {
        let mut center = [0.0; 3];
        center[self.axis().int()] = 1.5 * self.sign().float();
        let (ax1, ax2) = self.axis().perpendiculars();
        let mut ret = [center; 4];
        let mut i = 0;
        for &u in &[-0.3, 0.3] {
            for &v in &[-0.3, 0.3] {
                ret[i][ax1.int()] = u + self.piece()[ax1].float();
                ret[i][ax2.int()] = v + self.piece()[ax2].float();
                i += 1;
            }
        }
        ret
    }
    pub fn iter() -> impl Iterator<Item = Self> {
        Piece::iter().flat_map(Piece::stickers)
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Axis {
    X = 0,
    Y = 1,
    Z = 2,
}
impl Axis {
    pub fn int(self) -> usize {
        match self {
            Self::X => 0,
            Self::Y => 1,
            Self::Z => 2,
        }
    }
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
    pub fn iter() -> impl Iterator<Item = &'static Axis> {
        [Axis::X, Axis::Y, Axis::Z].iter()
    }
}

/// A face of the puzzle.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Face {
    axis: Axis,
    sign: Sign,
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
    pub fn new(axis: Axis, sign: Sign) -> Self {
        Self { axis, sign }
    }
    pub fn center(self) -> Piece {
        let mut ret = Piece::core();
        ret[self.axis] = self.sign;
        ret
    }
    pub fn center_sticker(self) -> Sticker {
        Sticker {
            piece: self.center(),
            axis: self.axis,
        }
    }
    pub fn color(self) -> [f32; 4] {
        match (self.axis, self.sign) {
            // Right = red
            (Axis::X, Sign::Pos) => [0.8, 0.0, 0.0, 1.0],
            // Left = orange
            (Axis::X, Sign::Neg) => [0.6, 0.2, 0.0, 1.0],
            // Up = white
            (Axis::Y, Sign::Pos) => [0.8, 0.8, 0.8, 1.0],
            // Down = yellow
            (Axis::Y, Sign::Neg) => [0.8, 0.8, 0.0, 1.0],
            // Front = green
            (Axis::Z, Sign::Pos) => [0.0, 0.8, 0.0, 1.0],
            // Back = blue
            (Axis::Z, Sign::Neg) => [0.0, 0.4, 0.8, 1.0],
            // Invalid
            (_, Sign::Zero) => panic!("Invalid face"),
        }
    }
    pub fn perpendiculars(self) -> (Axis, Axis) {
        let (ax1, ax2) = self.axis.perpendiculars();
        match self.sign {
            Sign::Neg => (ax2, ax1),
            Sign::Zero => panic!("Invalid face"),
            Sign::Pos => (ax1, ax2),
        }
    }
    pub fn iter() -> impl Iterator<Item = Self> {
        Axis::iter().flat_map(|&axis| Sign::iter().map(move |&sign| Self { axis, sign }))
    }
    pub fn stickers(self) -> impl Iterator<Item = Sticker> + 'static {
        let mut piece = self.center();
        let axis = self.axis;
        let (ax1, ax2) = self.axis.perpendiculars();
        Sign::iter().flat_map(move |&u| {
            piece[ax1] = u;
            Sign::iter().map(move |&v| {
                piece[ax2] = v;
                Sticker { piece, axis }
            })
        })
    }
}

/// An orientation of a cube (i.e. a single piece).
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Orientation([Face; 3]);
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
    #[must_use]
    pub fn rev(self) -> Self {
        let mut ret = Self::default();
        for &axis in Axis::iter() {
            ret[self[axis].axis] = Face::new(axis, self[axis].sign);
        }
        ret
    }
    /// Rotates this orientation 90 degrees from one axis to another.
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
