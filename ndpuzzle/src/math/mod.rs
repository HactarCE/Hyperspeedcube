//! N-dimensional vector math library.

pub use approx::{abs_diff_eq, AbsDiffEq};

#[macro_use]
mod impl_macros;
#[macro_use]
mod vector;
mod hyperplane;
mod matrix;
mod multivector;
pub mod permutations;
pub mod util;

pub use hyperplane::*;
pub use matrix::*;
pub use multivector::*;
pub use vector::*;

/// Small floating-point value used for comparisons and tiny offsets.
pub const EPSILON: f32 = 0.00001;

/// Compares two numbers, but considers them equal if they are separated by less
/// than `EPSILON`.
pub fn abs_diff_cmp<T: AbsDiffEq<Epsilon = f32> + PartialOrd>(a: &T, b: &T) -> std::cmp::Ordering {
    if abs_diff_eq!(a, b, epsilon = EPSILON) {
        std::cmp::Ordering::Equal
    } else if a < b {
        std::cmp::Ordering::Less
    } else {
        std::cmp::Ordering::Greater
    }
}
