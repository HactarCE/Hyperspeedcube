use std::path::{Path, PathBuf};
use std::sync::{mpsc, Arc};

use eyre::Result;
use itertools::Itertools;
use parking_lot::Mutex;

use super::{LibraryCommand, LibraryDb};
use crate::builder::ColorSystemBuilder;
use crate::lua::{LuaLoader, LuaLogger, PuzzleGeneratorSpec, PuzzleSpec};
use crate::{LuaLogLine, Puzzle, TaskHandle};

/// Handle to a library of puzzles.
///
/// All Lua execution and puzzle construction happens asynchronously on other
/// threads.
#[derive(Debug)]
pub struct Library {
    cmd_tx: mpsc::Sender<LibraryCommand>,
    log_rx: mpsc::Receiver<LuaLogLine>,
    db: Arc<Mutex<LibraryDb>>,
}

impl Default for Library {
    fn default() -> Self {
        Self::new()
    }
}

impl Library {
    /// Constructs a new puzzle library with its own Lua instance.
    pub fn new() -> Self {
        let (cmd_tx, cmd_rx) = mpsc::channel();
        let (logger, log_rx) = LuaLogger::new();

        let db = LibraryDb::new();
        let loader = LuaLoader::new(Arc::clone(&db), logger);

        std::thread::spawn(move || {
            for command in cmd_rx {
                match command {
                    LibraryCommand::ReadDirectory {
                        directory,
                        progress,
                    } => {
                        loader.db.lock().read_directory(&directory);
                        progress.complete(());
                    }

                    LibraryCommand::ReadFile {
                        filename,
                        path,
                        progress,
                    } => {
                        loader.db.lock().read_file(filename, path);
                        progress.complete(());
                    }

                    LibraryCommand::LoadFiles { progress } => {
                        loader.load_all_files();
                        progress.complete(());
                    }

                    LibraryCommand::BuildPuzzle { id, progress } => {
                        progress.complete(loader.build_puzzle(&id));
                    }
                }
            }
        });

        Library { cmd_tx, log_rx, db }
    }
    /// Sends a command to the [`LuaLoader`], which is on another thread.
    fn send_command(&self, command: LibraryCommand) {
        self.cmd_tx
            .send(command)
            .expect("error sending library command to loader thread");
    }

    /// Returns an iterator over all pending log lines. The iterator never
    /// blocks waiting for more log lines; when there are no more, it stops.
    pub fn pending_log_lines(&self) -> impl '_ + Iterator<Item = LuaLogLine> {
        self.log_rx.try_iter()
    }

    /// Adds a file to the Lua library. It will not immediately be loaded.
    ///
    /// If the filename conflicts with an existing one, then the existing file
    /// will be unloaded and overwritten.
    pub fn add_file(&self, filename: String, path: Option<PathBuf>, contents: String) {
        self.db.lock().add_file(filename, path, contents);
    }
    /// Reads a file from the disk and adds it to the Lua library. It will not
    /// immediately be loaded. Logs an error if the file could not be read.
    ///
    /// If the filename conflicts with an existing one, then the existing file
    /// will be overwritten.
    pub fn read_file(&self, filename: String, path: PathBuf) -> TaskHandle<()> {
        let task = TaskHandle::new();
        self.send_command(LibraryCommand::ReadFile {
            filename,
            path,
            progress: task.clone(),
        });
        task
    }
    /// Reads a directory recursively and adds all files ending in `.lua` to the
    /// Lua library. They will not immediately be loaded.
    ///
    /// If any filename conflicts with an existing one, then the existing file
    /// will be overwritten.
    pub fn read_directory(&self, directory: &Path) -> TaskHandle<()> {
        let task = TaskHandle::new();
        self.send_command(LibraryCommand::ReadDirectory {
            directory: directory.to_owned(),
            progress: task.clone(),
        });
        task
    }
    /// Canonicalizes a relative file path to make a suitable filename.
    pub fn relative_path_to_filename(path: &Path) -> String {
        path.components()
            .map(|component| component.as_os_str().to_string_lossy())
            .join("/")
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
    /// Reads a directory recursively and adds all files ending in `.lua` to the
    /// Lua library, then loads them all. Lua execution happens asynchronously,
    /// so changes might not take effect immediately; use the returned
    /// [`TaskHandle`] to check progress.
    ///
    /// If any filename conflicts with an existing one, then the existing file
    /// will be overwritten.
    pub fn load_directory(&self, directory: &Path) -> TaskHandle<()> {
        // Commands are processed in the order they are received. This should
        // _technically_ be atomic and isn't, but it hardly matters.
        let _ = self.read_directory(directory);
        self.load_files()
    }

    /// Returns a list of loaded puzzles, including generated puzzles.
    pub fn puzzles(&self) -> Vec<Arc<PuzzleSpec>> {
        let single_puzzles = self.non_generated_puzzles();
        let puzzle_generators = self.puzzle_generators();
        let generated_puzzles = puzzle_generators
            .iter()
            .flat_map(|gen| gen.examples.values())
            .map(Arc::clone);
        itertools::chain(single_puzzles, generated_puzzles)
            .sorted()
            .collect()
    }
    /// Returns a list of loaded puzzles, not including generated puzzles.
    pub fn non_generated_puzzles(&self) -> Vec<Arc<PuzzleSpec>> {
        self.db.lock().puzzles.values().cloned().collect()
    }
    /// Returns a list of loaded puzzle generators.
    pub fn puzzle_generators(&self) -> Vec<Arc<PuzzleGeneratorSpec>> {
        self.db.lock().puzzle_generators.values().cloned().collect()
    }
    /// Returns a list of loaded color systems.
    pub fn color_systems(&self) -> Vec<Arc<ColorSystemBuilder>> {
        self.db.lock().color_systems.values().cloned().collect()
    }

    /// Builds a puzzle from a Lua specification.
    pub fn build_puzzle(&self, id: &str) -> TaskHandle<Result<Arc<Puzzle>>> {
        let task = TaskHandle::new();
        self.send_command(LibraryCommand::BuildPuzzle {
            id: id.to_string(),
            progress: task.clone(),
        });
        task
    }

    /// Creates a new library with a fresh Lua state.
    pub fn reset(&mut self) {
        *self = Library::new();
    }
}
