use parking_lot::{Mutex, MutexGuard};
use std::collections::HashMap;
use std::sync::{mpsc, Arc};

use eyre::Result;

use super::{LibraryCommand, PuzzleData};
use crate::lua::LuaLoader;
use crate::{LuaLogLine, Puzzle, TaskHandle};

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
    /// Constructs a new puzzle library with its own Lua instance.
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel();

        let puzzles = Arc::new(Mutex::new(HashMap::new()));
        let puzzles_ref2 = Arc::clone(&puzzles);

        std::thread::spawn(move || {
            let loader = LuaLoader::new();

            for command in rx {
                match command {
                    LibraryCommand::SetLogLineHandler { handler } => {
                        if let Err(e) = loader.set_log_line_handler(handler) {
                            log::error!("failed to set Lua log line handler: {e}");
                        }
                    }

                    LibraryCommand::AddFile { filename, contents } => {
                        if let Err(e) = loader.set_file_contents(&filename, Some(&contents)) {
                            log::error!("failed to add file {filename}: {e}");
                        }
                    }
                    LibraryCommand::RemoveFile { filename } => {
                        if let Err(e) = loader.set_file_contents(&filename, None) {
                            log::error!("failed to remove file {filename}: {e}");
                        }
                    }
                    LibraryCommand::LoadFiles { progress } => {
                        loader.load_all_files();
                        *puzzles_ref2.lock() = loader
                            .get_puzzle_data()
                            .into_iter()
                            .map(|data| (data.name.clone(), data))
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
    /// Sends a command to the [`LuaLoader`], which is on another thread.
    fn send_command(&self, command: LibraryCommand) {
        self.tx
            .send(command)
            .expect("error sending library command to loader thread");
    }

    /// Sets a callback to be run for log lines emitted by Lua code.
    pub fn set_log_line_handler(&self, handler: Box<dyn 'static + Send + Fn(LuaLogLine)>) {
        self.send_command(LibraryCommand::SetLogLineHandler { handler });
    }

    /// Adds a file to the Lua library. It will not immediately be loaded.
    ///
    /// If the filename conflicts with an existing one, then the existing file
    /// will be overwritten.
    pub fn add_file(&self, filename: String, contents: String) {
        self.send_command(LibraryCommand::AddFile { filename, contents });
    }
    /// Removes a file from the Lua library.
    pub fn remove_file(&self, filename: String) {
        self.send_command(LibraryCommand::RemoveFile { filename });
    }
    /// Loads all files that haven't been loaded yet. Lua execution happens
    /// asynchronously, so changes might not take effect immediately; use the
    /// returned [`TaskHandle`] to check progress.
    pub fn load_files(&self) -> TaskHandle<()> {
        let task = TaskHandle::new();
        self.send_command(LibraryCommand::LoadFiles {
            progress: task.clone(),
        });
        task
    }
    /// Returns the full list of loaded puzzles.
    pub fn puzzles(&self) -> MutexGuard<'_, HashMap<String, PuzzleData>> {
        self.puzzles.lock()
    }
    /// Builds a puzzle from a Lua specification.
    pub fn build_puzzle(&self, name: &str) -> TaskHandle<Result<Arc<Puzzle>>> {
        let task = TaskHandle::new();
        self.send_command(LibraryCommand::BuildPuzzle {
            name: name.to_string(),
            progress: task.clone(),
        });
        task
    }
}
