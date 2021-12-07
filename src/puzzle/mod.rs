//! Common types and traits used for any puzzle.

use std::fmt;

pub mod controller;
pub mod rubiks3d;
pub mod rubiks4d;
pub mod rubiks4d_logfile;
pub mod sign;
pub mod traits;

pub use controller::*;
pub use rubiks3d::Rubiks3D;
pub use rubiks4d::Rubiks4D;
pub use sign::Sign;
pub use traits::*;

/// An enumeration of all puzzle types.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum PuzzleType {
    /// A 3D Rubik's cube.
    Rubiks3D,
    /// A 4D Rubik's cube.
    Rubiks4D,
}
impl Default for PuzzleType {
    fn default() -> Self {
        PuzzleType::Rubiks4D
    }
}
impl fmt::Display for PuzzleType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PuzzleType::Rubiks3D => write!(f, "Rubik's 3D"),
            PuzzleType::Rubiks4D => write!(f, "Rubik's 4D"),
        }
    }
}
impl PuzzleType {
    /// Creates a new puzzle of this type.
    pub fn new(self) -> PuzzleEnum {
        match self {
            Self::Rubiks3D => PuzzleEnum::Rubiks3D(Default::default()),
            Self::Rubiks4D => PuzzleEnum::Rubiks4D(Default::default()),
        }
    }
}

/// A PuzzleController of any puzzle type.
#[derive(PartialEq, Eq)]
pub enum PuzzleEnum {
    /// A 3D Rubik's cube.
    Rubiks3D(PuzzleController<Rubiks3D>),
    /// A 4D Rubik's cube.
    Rubiks4D(PuzzleController<Rubiks4D>),
}
impl From<PuzzleType> for PuzzleEnum {
    fn from(puzzle_type: PuzzleType) -> Self {
        puzzle_type.new()
    }
}
impl PuzzleEnum {
    /// Returns the PuzzleType of this puzzle.
    pub fn puzzle_type(&self) -> PuzzleType {
        match self {
            Self::Rubiks3D(_) => PuzzleType::Rubiks3D,
            Self::Rubiks4D(_) => PuzzleType::Rubiks4D,
        }
    }
    /// Advance to the next frame, using the given time delta between this frame
    /// and the last.
    pub fn advance(&mut self, delta: std::time::Duration) {
        match self {
            Self::Rubiks3D(cube) => cube.advance(delta),
            Self::Rubiks4D(cube) => cube.advance(delta),
        }
    }

    /// Returns whether the puzzle has been modified since the last time the log
    /// file was saved.
    pub fn needs_save(&self) -> bool {
        match self {
            PuzzleEnum::Rubiks3D(cube) => cube.needs_save,
            PuzzleEnum::Rubiks4D(cube) => cube.needs_save,
        }
    }

    /// Undoes one twist.
    pub fn undo(&mut self) {
        match self {
            Self::Rubiks3D(cube) => cube.undo(),
            Self::Rubiks4D(cube) => cube.undo(),
        }
    }
    /// Redoes one twist.
    pub fn redo(&mut self) {
        match self {
            Self::Rubiks3D(cube) => cube.redo(),
            Self::Rubiks4D(cube) => cube.redo(),
        }
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
impl fmt::Display for TwistDirection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TwistDirection::CW => Ok(()),
            TwistDirection::CCW => write!(f, "'"),
        }
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
    /// Returns the sign of this rotation, according to the mathematical
    /// convention of counterclockwise being positive and clockwise being
    /// negative.
    pub fn sign(self) -> Sign {
        match self {
            Self::CW => Sign::Neg,
            Self::CCW => Sign::Pos,
        }
    }
}
impl From<TwistDirection> for Sign {
    fn from(direction: TwistDirection) -> Self {
        direction.sign()
    }
}
