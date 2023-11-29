use std::sync::Arc;

use eyre::Result;

use crate::{Puzzle, TaskHandle};

pub(super) enum LibraryCommand {
    AddFile {
        filename: String,
        contents: String,
    },
    LoadFiles {
        progress: TaskHandle<()>,
    },
    BuildPuzzle {
        name: String,
        progress: TaskHandle<Result<Arc<Puzzle>>>,
    },
}
