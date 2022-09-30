//! N-dimensional vector math library.

#![warn(clippy::if_then_some_else_none, missing_docs)]

#[macro_use]
mod impl_macros;
#[macro_use]
mod vector;
mod hyperplane;
mod matrix;
pub mod permutations;
mod rotor;

pub use hyperplane::*;
pub use matrix::*;
pub use rotor::*;
pub use vector::*;
