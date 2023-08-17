//! Conformal geometric algebra.
//!
//! https://en.wikipedia.org/wiki/Conformal_geometric_algebra

use crate::Sign;

mod axes;
mod blade;
mod isometry;
mod multivector;
mod point;
mod tangent;
mod term;

pub use axes::Axes;
pub use blade::{Blade, MismatchedGrade};
pub use isometry::Isometry;
pub use multivector::{AsMultivector, Multivector};
pub use point::{Point, ToConformalPoint};
pub use tangent::TangentSpace;
pub use term::Term;

/// Position of a point relative to an oriented manifold that divides space.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum PointWhichSide {
    /// The point is on the manifold between inside and outside.
    On,
    /// The point is on the "inside" space relative to the manifold.
    Inside,
    /// The point is on the "outside" space relative to the manifold.
    Outside,
}
impl std::ops::Mul<Sign> for PointWhichSide {
    type Output = Self;

    fn mul(self, rhs: Sign) -> Self::Output {
        match rhs {
            Sign::Pos => self,
            Sign::Neg => match self {
                Self::On => Self::On,
                Self::Inside => Self::Outside,
                Self::Outside => Self::Inside,
            },
        }
    }
}

#[cfg(test)]
mod tests;
