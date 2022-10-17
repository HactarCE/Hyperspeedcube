//! Common types and traits used for any puzzle.

pub mod controller;
mod render;
mod types;

pub use controller::*;
pub use ndpuzzle::puzzle::*; // TODO: maybe don't?
use render::PuzzleRenderCache;
pub use types::PUZZLE_REGISTRY;

pub mod traits {
    pub use super::{PuzzleInfo, PuzzleState, PuzzleType};
}
