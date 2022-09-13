//! N-dimensional vector math library.

#![warn(clippy::if_then_some_else_none, missing_docs)]

#[macro_use]
mod vector;
mod matrix;
pub mod permutations;

pub use matrix::*;
pub use vector::*;
