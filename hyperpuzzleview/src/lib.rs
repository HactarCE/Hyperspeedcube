//! Puzzle user interface state manager, to ensure consistent feel across
//! frontends.

mod animations;
mod simulation;
mod styles;
mod util;
mod view;

pub use simulation::PuzzleSimulation;
pub use view::{DragState, HoverMode, PuzzleFiltersState, PuzzleView, PuzzleViewInput};

/// Speed multiplier for twists using mouse dragging.
///
/// TODO: reword this and move it to preferences
pub const TWIST_DRAG_SPEED: f32 = 2.0;
