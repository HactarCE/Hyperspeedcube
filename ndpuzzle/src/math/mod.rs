//! N-dimensional vector math library.

#![warn(clippy::if_then_some_else_none, missing_docs)]

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
