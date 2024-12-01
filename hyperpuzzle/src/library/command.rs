use std::path::PathBuf;
use std::sync::Arc;

use eyre::Result;

use crate::{Puzzle, TaskHandle};

/// Command sent to the thread with the Lua interpreter.
pub(crate) enum LibraryCommand {
    /// Read Lua files from a directory and add them to the library.
    ReadDirectory {
        directory: PathBuf,
        progress: TaskHandle<()>,
    },

    /// Read a Lua file and add it to the library.
    ReadFile {
        filename: String,
        path: PathBuf,
        progress: TaskHandle<()>,
    },

    /// Execute all Lua files that haven't been executed yet.
    LoadFiles { progress: TaskHandle<()> },

    /// Build a puzzle that has already been loaded from a Lua file.
    BuildPuzzle {
        id: String,
        progress: TaskHandle<Result<Arc<Puzzle>>>,
    },
}
