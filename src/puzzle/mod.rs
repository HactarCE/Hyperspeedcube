//! Common types and traits used for any puzzle.

use std::ops::{Add, Mul, Neg};

pub mod controller;
pub mod rubiks3d;
pub mod traits;

pub use controller::*;
pub use rubiks3d::Rubiks3D;
pub use traits::*;

/// An enumeration of all puzzle types.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum PuzzleType {
    /// A 3D Rubik's cube.
    Rubiks3D,
}
impl PuzzleType {
    /// Creates a new puzzle of this type.
    pub fn new(self) -> PuzzleEnum {
        match self {
            Self::Rubiks3D => PuzzleEnum::Rubiks3D(Default::default()),
        }
    }
}

/// A PuzzleController of any puzzle type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PuzzleEnum {
    /// A 3D Rubik's cube.
    Rubiks3D(PuzzleController<Rubiks3D>),
}
impl From<PuzzleType> for PuzzleEnum {
    fn from(puzzle_type: PuzzleType) -> Self {
        puzzle_type.new()
    }
}
impl PuzzleEnum {
    /// Returns the PuzzleType of this puzzle.
    fn puzzle_type(&self) -> PuzzleType {
        match self {
            Self::Rubiks3D(_) => PuzzleType::Rubiks3D,
        }
    }
}

/// Positive, negative, or zero.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Sign {
    /// Negative.
    Neg = -1,
    /// Zero.
    Zero = 0,
    /// Positive.
    Pos = 1,
}
impl Default for Sign {
    fn default() -> Self {
        Self::Zero
    }
}
impl From<TwistDirection> for Sign {
    fn from(direction: TwistDirection) -> Self {
        direction.sign()
    }
}
impl Neg for Sign {
    type Output = Self;
    fn neg(self) -> Self {
        match self {
            Self::Neg => Self::Pos,
            Self::Zero => Self::Zero,
            Self::Pos => Self::Neg,
        }
    }
}
impl Mul<Sign> for Sign {
    type Output = Self;
    fn mul(self, rhs: Self) -> Self {
        match self {
            Self::Neg => -rhs,
            Self::Zero => Self::Zero,
            Self::Pos => rhs,
        }
    }
}
impl Add<Sign> for Sign {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        match self {
            Self::Neg => match rhs {
                Self::Neg => panic!("Too negative"),
                Self::Zero => Self::Neg,
                Self::Pos => Self::Zero,
            },
            Self::Zero => rhs,
            Self::Pos => match rhs {
                Self::Neg => Self::Zero,
                Self::Zero => Self::Pos,
                Self::Pos => panic!("Too positive"),
            },
        }
    }
}
impl Sign {
    /// Returns an integer representation of this sign (either -1, 0, or 1).
    pub fn int(self) -> isize {
        match self {
            Self::Neg => -1,
            Self::Zero => 0,
            Self::Pos => 1,
        }
    }
    /// Returns a floating-point representation of this sign (either -1.0, 0.0,
    /// or 1.0).
    pub fn float(self) -> f32 {
        self.int() as f32
    }
    /// Returns the absolute value of the integer representation of this sign (either 0 or 1).
    pub fn abs(self) -> usize {
        match self {
            Self::Neg | Self::Pos => 1,
            Self::Zero => 0,
        }
    }
    /// Returns true if this is Sign::Zero or false otherwise.
    pub fn is_zero(self) -> bool {
        self == Self::Zero
    }
    /// Returns false if this is Sign::Zero or true otherwise.
    pub fn is_nonzero(self) -> bool {
        self != Self::Zero
    }
    /// Returns an iterator over all Sign variants.
    pub fn iter() -> impl Iterator<Item = &'static Self> {
        [Self::Neg, Self::Zero, Self::Pos].iter()
    }
}

/// A rotation direction; clockwise or counterclockwise.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum TwistDirection {
    /// Clockwise.
    CW,
    /// Counterclockwise.
    CCW,
}
impl Default for TwistDirection {
    fn default() -> Self {
        Self::CW
    }
}
impl TwistDirection {
    /// Returns the reverse direction.
    #[must_use]
    pub fn rev(self) -> Self {
        match self {
            Self::CW => Self::CCW,
            Self::CCW => Self::CW,
        }
    }
    /// Returns the sign of this rotation, according to the speedsolving
    /// convention of clockwise being positive and counterclockwise being
    /// negative.
    pub fn sign(self) -> Sign {
        match self {
            Self::CW => Sign::Pos,
            Self::CCW => Sign::Neg,
        }
    }
}
