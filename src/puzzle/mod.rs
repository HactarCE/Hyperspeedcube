//! Common types and traits used for any puzzle.

#[macro_use]
mod common;

pub mod traits {
    pub use super::common::{PuzzleInfo, PuzzleState, PuzzleType};
}

pub mod controller;
pub mod geometry;
pub mod rubiks_3d;
pub mod rubiks_4d;

pub use common::*;
pub use controller::*;
pub use geometry::*;
pub use rubiks_3d::Rubiks3D;
pub use rubiks_4d::Rubiks4D;
