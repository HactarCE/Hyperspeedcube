//! Common types and traits used for any puzzle.

use std::fmt;

// #[macro_use]
// mod types;
#[macro_use]
pub mod traits;

pub mod types {}

pub mod controller;
mod generic;
pub mod geometry;
mod metric;
pub mod rubiks_3d;
pub mod rubiks_4d;
pub mod sign;

pub use controller::*;
pub use generic::*;
pub use geometry::*;
pub use metric::TwistMetric;
pub use rubiks_3d::Rubiks3D;
pub use rubiks_4d::Rubiks4D;
pub use sign::Sign;
pub use traits::*;
// pub use types::PuzzleType;
