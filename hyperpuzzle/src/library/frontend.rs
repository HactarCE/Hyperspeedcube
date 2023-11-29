use parking_lot::{Mutex, MutexGuard};
use std::collections::HashMap;
use std::sync::{mpsc, Arc};

use eyre::Result;

use super::{LibraryCommand, PuzzleData};
use crate::{lua::LuaLoader, Puzzle, TaskHandle};

/// Handle to a library of puzzles. This type is cheap to `Clone` (just an
/// `mpsc::Sender` and `Arc`).
///
/// All Lua execution and puzzle construction happens asynchronously on other
/// threads.
#[derive(Debug, Clone)]
pub struct Library {
    tx: mpsc::Sender<LibraryCommand>,
    puzzles: Arc<Mutex<HashMap<String, PuzzleData>>>,
}

impl Default for Library {
    fn default() -> Self {
        Self::new()
    }
}

impl Library {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel();

        let puzzles = Arc::new(Mutex::new(HashMap::new()));
        let puzzles_ref2 = Arc::clone(&puzzles);

        std::thread::spawn(move || {
            let loader = LuaLoader::new();

            for command in rx {
                match command {
                    LibraryCommand::AddFile { filename, contents } => {
                        if let Err(e) = loader.set_file_contents(&filename, &contents) {
                            log::error!("failed to load file {filename}: {e}");
                        }
                    }
                    LibraryCommand::LoadFiles { progress } => {
                        loader.load_all_files();
                        *puzzles_ref2.lock() = loader
                            .get_all_puzzle_names()
                            .into_iter()
                            .map(|name| (name.clone(), PuzzleData { name }))
                            .collect();
                        progress.complete(());
                    }
                    LibraryCommand::BuildPuzzle { name, progress } => {
                        progress.complete(loader.build_puzzle(&name));
                    }
                }
            }
        });

        Library { tx, puzzles }
    }

    pub fn add_file(&self, filename: String, contents: String) {
        self.send_command(LibraryCommand::AddFile { filename, contents });
    }
    pub fn load_files(&self) -> TaskHandle<()> {
        let task = TaskHandle::new();
        self.send_command(LibraryCommand::LoadFiles {
            progress: task.clone(),
        });
        task
    }
    pub fn puzzles(&self) -> MutexGuard<'_, HashMap<String, PuzzleData>> {
        self.puzzles.lock()
    }
    pub fn build_puzzle(&self, name: &str) -> TaskHandle<Result<Arc<Puzzle>>> {
        let task = TaskHandle::new();
        self.send_command(LibraryCommand::BuildPuzzle {
            name: name.to_string(),
            progress: task.clone(),
        });
        task
    }

    fn send_command(&self, command: LibraryCommand) {
        self.tx
            .send(command)
            .expect("error sending library command to loader thread");
    }
}
