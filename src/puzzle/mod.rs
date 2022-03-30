//! Common types and traits used for any puzzle.

use std::fmt;

#[macro_use]
mod types;
#[macro_use]
pub mod traits;

pub mod controller;
mod generic;
mod metric;
pub mod rubiks3d;
pub mod rubiks4d;
pub mod rubiks4d_logfile;
pub mod sign;

pub use controller::{PuzzleController, PuzzleControllerTrait, ScrambleState};
pub use generic::*;
pub use metric::TwistMetric;
pub use rubiks3d::Rubiks3D;
pub use rubiks4d::Rubiks4D;
pub use sign::Sign;
pub use traits::*;
pub use types::PuzzleType;

/// Rotation direction; clockwise or counterclockwise.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum TwistDirection2D {
    /// Clockwise.
    CW,
    /// Counterclockwise.
    CCW,
}
impl Default for TwistDirection2D {
    fn default() -> Self {
        Self::CW
    }
}
impl fmt::Display for TwistDirection2D {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TwistDirection2D::CW => Ok(()),
            TwistDirection2D::CCW => write!(f, "'"),
        }
    }
}
impl TwistDirection2D {
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
impl From<TwistDirection2D> for Sign {
    fn from(direction: TwistDirection2D) -> Self {
        direction.sign()
    }
}
