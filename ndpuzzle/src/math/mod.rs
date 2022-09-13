//! N-dimensional vector math library.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![warn(clippy::if_then_some_else_none, missing_docs)]

#[macro_use]
mod vector;
mod matrix;
pub mod permutations;

pub use matrix::*;
pub use vector::*;
