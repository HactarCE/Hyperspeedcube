//! Common types and traits used for any puzzle.

#[macro_use]
mod common;

pub mod controller;
pub mod geometry;
pub mod notation;
pub mod rubiks_3d;
pub mod rubiks_4d;
mod types;

pub use common::*;
pub use controller::*;
pub use geometry::*;
pub use notation::*;
pub use rubiks_3d::Rubiks3D;
pub use rubiks_4d::Rubiks4D;
pub use types::PUZZLE_REGISTRY;

pub mod traits {
    pub use super::{PuzzleInfo, PuzzleState, PuzzleType};
}
