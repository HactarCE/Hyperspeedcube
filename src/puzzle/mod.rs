//! Common types and traits used for any puzzle.

use std::fmt;

#[macro_use]
mod types;
#[macro_use]
pub mod traits;

pub mod common_4d;
mod generic;
pub mod geometry;
mod metric;
pub mod rubiks24;
pub mod rubiks33;
pub mod rubiks34;
pub mod sign;

pub use generic::*;
pub use geometry::*;
pub use metric::TwistMetric;
pub use rubiks24::Rubiks24;
pub use rubiks33::Rubiks33;
pub use rubiks34::Rubiks34;
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
impl std::ops::Mul<Sign> for TwistDirection2D {
    type Output = Self;

    fn mul(self, rhs: Sign) -> Self::Output {
        match rhs {
            Sign::Neg => self.rev(),
            Sign::Zero => panic!("cannot multiply twist by zero"),
            Sign::Pos => self,
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
