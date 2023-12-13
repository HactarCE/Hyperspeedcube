use std::sync::Arc;

use eyre::Result;

use crate::{LuaLogLine, Puzzle, TaskHandle};

/// Command sent to the thread with the Lua interpreter.
pub(super) enum LibraryCommand {
    /// Set a callback to be run for log lines emitted by Lua code.
    SetLogLineHandler {
        handler: Box<dyn 'static + Send + Fn(LuaLogLine)>,
    },

    /// Add a Lua file to the library.
    AddFile { filename: String, contents: String },
    /// Remove a Lua file from the library.
    RemoveFile { filename: String },
    /// Execute all Lua files that haven't been executed yet.
    LoadFiles { progress: TaskHandle<()> },

    /// Build a puzzle that has already been loaded from a Lua file.
    BuildPuzzle {
        name: String,
        progress: TaskHandle<Result<Arc<Puzzle>>>,
    },
}
