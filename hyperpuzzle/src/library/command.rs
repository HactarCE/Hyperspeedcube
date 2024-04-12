use std::sync::Arc;

use eyre::Result;

use crate::{Puzzle, TaskHandle};

/// Command sent to the thread with the Lua interpreter.
pub(crate) enum LibraryCommand {
    /// Execute all Lua files that haven't been executed yet.
    LoadFiles { progress: TaskHandle<()> },

    /// Build a puzzle that has already been loaded from a Lua file.
    BuildPuzzle {
        id: String,
        progress: TaskHandle<Result<Arc<Puzzle>>>,
    },
}
