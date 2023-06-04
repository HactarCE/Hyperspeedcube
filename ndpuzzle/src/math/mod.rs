//! N-dimensional vector math library.

pub use approx::AbsDiffEq;
use num_traits::Zero;

#[macro_use]
mod impl_macros;
#[macro_use]
mod vector;
pub mod cga;
mod matrix;
pub mod permutations;
mod sign;
pub mod util;

pub use crate::{Float, EPSILON};
pub use cga::{AsMultivector, ToConformalPoint};
pub use matrix::*;
pub use sign::*;
pub use vector::*;

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

/// Compares two numbers, but considers them equal if they are separated by less
/// than `EPSILON`.
pub fn approx_eq<T: AbsDiffEq<Epsilon = Float>>(a: &T, b: &T) -> bool {
    approx::abs_diff_eq!(a, b, epsilon = EPSILON)
}

/// Compares two numbers, but considers them equal if they are separated by less
/// than `EPSILON`.
pub fn approx_cmp<T: AbsDiffEq<Epsilon = Float> + PartialOrd>(a: &T, b: &T) -> std::cmp::Ordering {
    if approx_eq(a, b) {
        std::cmp::Ordering::Equal
    } else if a < b {
        std::cmp::Ordering::Less
    } else {
        std::cmp::Ordering::Greater
    }
}
/// Returns whether one number is less than another by at least `EPSILON`.
pub fn approx_lt<T: AbsDiffEq<Epsilon = Float> + PartialOrd>(a: &T, b: &T) -> bool {
    a < b && !approx_eq(a, b)
}
/// Returns whether one number is greater than another by at least `EPSILON`.
pub fn approx_gt<T: AbsDiffEq<Epsilon = Float> + PartialOrd>(a: &T, b: &T) -> bool {
    a > b && !approx_eq(a, b)
}

/// Returns whether `x` has an absolute value greater than `EPSILON`.
pub fn is_approx_nonzero<T: AbsDiffEq<Epsilon = Float> + Zero>(x: &T) -> bool {
    !approx_eq(x, &T::zero())
}
/// Returns whether `x` is less than `-EPSILON`.
pub fn is_approx_negative<T: AbsDiffEq<Epsilon = Float> + PartialOrd + Zero>(x: &T) -> bool {
    approx_lt(x, &T::zero())
}
/// Returns whether `x` is greater than `EPSILON`.
pub fn is_approx_positive<T: AbsDiffEq<Epsilon = Float> + PartialOrd + Zero>(x: &T) -> bool {
    approx_gt(x, &T::zero())
}
