use anyhow::{anyhow, Context, Result};
use parking_lot::Mutex;

use super::TaskHandle;

#[derive(Debug, Clone)]
pub struct PuzzleLibrary();

impl Default for PuzzleLibrary {
    fn default() -> Self {
        Self::new()
    }
}

impl PuzzleLibrary {
    pub fn new() -> Self {
        todo!()
    }

    pub fn load_file(
        &self,
        filename: impl Into<String>,
        contents: impl Into<String>,
    ) -> TaskHandle<Result<()>> {
        todo!()
    }
}

enum Command {
    LoadFile {
        filename: String,
        contents: String,
        progress: Mutex<TaskHandle<Result<()>>>,
    },
}
