//! Common types and traits used for any puzzle.

use std::fmt;

#[macro_use]
mod types;
#[macro_use]
pub mod traits;

pub mod commands;
pub mod controller;
mod generic;
pub mod rubiks3d;
pub mod rubiks4d;
pub mod rubiks4d_logfile;
pub mod sign;

pub use commands::{Command, FaceId, LayerMask, PieceTypeId};
pub use controller::{PuzzleController, ScrambleState};
pub use generic::*;
pub use rubiks3d::Rubiks3D;
pub use rubiks4d::Rubiks4D;
pub use sign::Sign;
pub use traits::*;
pub use types::PuzzleType;

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
