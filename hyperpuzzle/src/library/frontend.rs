use std::path::{Path, PathBuf};
use std::sync::{mpsc, Arc};

use eyre::Result;
use itertools::Itertools;
use parking_lot::Mutex;

use super::{LibraryCommand, LibraryDb, LibraryFile};
use crate::lua::{LuaLoader, LuaLogger, PuzzleParams};
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
        let db_ref2 = Arc::clone(&db);

        std::thread::spawn(move || {
            let loader = LuaLoader::new(db, logger);

            for command in cmd_rx {
                match command {
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

        let db = db_ref2;
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
    pub fn read_file(&self, filename: String, file_path: &Path) {
        let file_path = file_path.strip_prefix(".").unwrap_or(file_path);
        match std::fs::read_to_string(file_path) {
            Ok(contents) => self.add_file(filename, Some(file_path.to_path_buf()), contents),
            Err(e) => log::error!("error loading {file_path:?}: {e}"),
        }
    }
    /// Reads a directory recursively and adds all files ending in `.lua` to the
    /// Lua library. They will not immediately be loaded.
    ///
    /// If any filename conflicts with an existing one, then the existing file
    /// will be overwritten.
    pub fn read_directory(&self, directory: &Path) {
        for entry in walkdir::WalkDir::new(directory).follow_links(true) {
            match entry {
                Ok(entry) => {
                    let path = entry.path();
                    if path.extension().is_some_and(|ext| ext == "lua") {
                        let relative_path = path.strip_prefix(directory).unwrap_or(path);
                        let name = Self::relative_path_to_filename(relative_path);
                        self.read_file(name, path);
                    }
                }
                Err(e) => log::warn!("error reading filesystem entry: {e:?}"),
            }
        }
    }
    /// Canonicalizes a relative file path to make a suitable filename.
    pub fn relative_path_to_filename(path: &Path) -> String {
        path.components()
            .map(|component| component.as_os_str().to_string_lossy())
            .join("/")
    }
    /// Unloads and removes a file from the Lua library.
    pub fn remove_file(&self, filename: &str) {
        self.db.lock().remove_file(&filename);
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
        self.read_directory(directory);
        self.load_files()
    }
    /// Returns the full list of loaded puzzles.
    pub fn puzzles(&self) -> Vec<Arc<PuzzleParams>> {
        self.db
            .lock()
            .puzzles
            .iter()
            .filter_map(|(id, file)| {
                let Some(load_result) = file.as_completed() else {
                    log::error!("file {:?} owns puzzle {id:?} but is unloaded", file.name);
                    return None;
                };
                let Some(puzzle) = load_result.puzzles.get(id) else {
                    log::error!("puzzle {id:?} not found in file {:?}", file.name);
                    return None;
                };
                Some(Arc::clone(&puzzle.params))
            })
            .sorted_by_cached_key(|params| params.name.clone())
            .collect_vec()
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

    /// Returns the file that defined the puzzle with the given ID.
    pub fn file_containing_puzzle(&self, id: &str) -> Option<Arc<LibraryFile>> {
        self.db.lock().puzzles.get(id).map(Arc::clone)
    }
}
