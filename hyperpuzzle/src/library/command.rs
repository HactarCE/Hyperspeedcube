use std::sync::Arc;

use eyre::Result;

use crate::{Puzzle, TaskHandle};

pub(super) enum LibraryCommand {
    LoadFile {
        filename: String,
        contents: String,
        progress: TaskHandle<Result<()>>,
    },
    ConstructPuzzle {
        name: String,
        progress: TaskHandle<Result<Arc<Puzzle>>>,
    },
}
