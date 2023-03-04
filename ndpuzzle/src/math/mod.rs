//! N-dimensional vector math library.

pub use approx::AbsDiffEq;

#[macro_use]
mod impl_macros;
#[macro_use]
mod vector;
pub mod cga;
mod hyperplane;
mod matrix;
mod multivector;
pub mod permutations;
mod subspace;
pub mod util;

pub use hyperplane::*;
pub use matrix::*;
pub use multivector::*;
pub use subspace::*;
pub use vector::*;

/// Small floating-point value used for comparisons and tiny offsets.
pub const EPSILON: f32 = 0.0001;

/// Compares two numbers, but considers them equal if they are separated by less
/// than `EPSILON`.
pub fn approx_eq<T: AbsDiffEq<Epsilon = f32>>(a: &T, b: &T) -> bool {
    approx::abs_diff_eq!(a, b, epsilon = EPSILON)
}

/// Compares two numbers, but considers them equal if they are separated by less
/// than `EPSILON`.
pub fn approx_cmp<T: AbsDiffEq<Epsilon = f32> + PartialOrd>(a: &T, b: &T) -> std::cmp::Ordering {
    if approx_eq(a, b) {
        std::cmp::Ordering::Equal
    } else if a < b {
        std::cmp::Ordering::Less
    } else {
        std::cmp::Ordering::Greater
    }
}
