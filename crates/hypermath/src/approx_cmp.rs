//! Approximate comparison functions that automatically use [`EPSILON`].

pub use approx::AbsDiffEq;
use num_traits::Zero;

use crate::{EPSILON, Float};

/// Compares two numbers, but considers them equal if they are separated by less
/// than `EPSILON`.
///
/// Handles infinity specially.
pub fn approx_eq<T: AbsDiffEq<Epsilon = Float>>(a: &T, b: &T) -> bool {
    // use native float equality to handle infinities
    a == b || approx::abs_diff_eq!(a, b, epsilon = EPSILON)
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

/// Returns whether one number is less than another or within `EPSILON` of it.
pub fn approx_lt_eq<T: AbsDiffEq<Epsilon = Float> + PartialOrd>(a: &T, b: &T) -> bool {
    a < b || approx_eq(a, b)
}
/// Returns whether one number is greater than another or within `EPSILON` of
/// it.
pub fn approx_gt_eq<T: AbsDiffEq<Epsilon = Float> + PartialOrd>(a: &T, b: &T) -> bool {
    a > b || approx_eq(a, b)
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
/// Returns `f` as an integer if it is approximately equal to one.
pub fn to_approx_integer(f: Float) -> Option<i64> {
    Some(f as i64).filter(|&i| approx_eq(&f, &(i as Float)))
}
