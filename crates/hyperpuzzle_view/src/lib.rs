//! Puzzle user interface state manager, to ensure consistent feel across
//! frontends.

mod action;
mod animations;
mod replay_event;
mod simulation;
mod styles;
mod util;
mod view;

pub use action::Action;
use action::UndoBehavior;
pub use replay_event::ReplayEvent;
pub use simulation::PuzzleSimulation;
pub use view::{
    DragState, HoverMode, NdEuclidViewState, PuzzleFiltersState, PuzzleView, PuzzleViewInput,
};

/// Speed multiplier for twists using mouse dragging.
///
/// TODO: reword this and move it to preferences
pub const TWIST_DRAG_SPEED: f32 = 2.0;
