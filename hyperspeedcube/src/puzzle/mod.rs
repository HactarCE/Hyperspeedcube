//! Common types and traits used for any puzzle.

pub mod controller;
pub mod rubiks_3d;
pub mod rubiks_4d;
mod types;

pub use controller::*;
pub use ndpuzzle::puzzle::*; // TODO: maybe don't?
pub use rubiks_3d::Rubiks3D;
pub use rubiks_4d::Rubiks4D;
pub use types::PUZZLE_REGISTRY;

pub mod traits {
    pub use super::{PuzzleInfo, PuzzleState, PuzzleType};
}
