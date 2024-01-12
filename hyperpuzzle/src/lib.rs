//! Multidimensional twisty puzzle generator and simulator backend.

mod library;
mod lua;
mod puzzle;
mod task;

pub use library::{Library, PuzzleData};
pub use lua::LuaLogLine;
pub use puzzle::*;
pub use task::TaskHandle;

/// Unsigned integer type used for [`LayerMask`].
pub type LayerMaskUint = u32;
