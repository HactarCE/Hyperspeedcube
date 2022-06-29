//! Common types and traits used for any puzzle.

use std::fmt;

// #[macro_use]
// mod types;
#[macro_use]
pub mod traits;

pub mod types {}

mod generic;
pub mod geometry;
mod metric;
// pub mod rubiks34;
pub mod controller;
pub mod rubiks_3d;
pub mod sign;

pub use controller::*;
pub use generic::*;
pub use geometry::*;
pub use metric::TwistMetric;
// pub use rubiks34::Rubiks34;
pub use rubiks_3d::Rubiks3D;
pub use sign::Sign;
pub use traits::*;
// pub use types::PuzzleType;
