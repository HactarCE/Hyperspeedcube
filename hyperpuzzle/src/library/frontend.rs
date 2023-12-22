use itertools::Itertools;
use parking_lot::{Mutex, MutexGuard};
use std::collections::HashMap;
use std::path::Path;
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
                            let e = e.wrap_err("failed to set Lua log line handler");
                            log::error!("{e:?}");
                        }
                    }

                    LibraryCommand::AddFile { filename, contents } => {
                        if let Err(e) = loader.set_file_contents(&filename, Some(&contents)) {
                            let e = e.wrap_err(format!("failed to add file {filename}"));
                            log::error!("{e:?}");
                        }
                    }
                    LibraryCommand::RemoveFile { filename } => {
                        if let Err(e) = loader.set_file_contents(&filename, None) {
                            let e = e.wrap_err(format!("failed to remove file {filename}"));
                            log::error!("{e:?}");
                        }
                    }
                    LibraryCommand::RemoveAllFiles => {
                        if let Err(e) = loader.remove_all_files() {
                            let e = e.wrap_err("failed to remove all files");
                            log::error!("{e:?}");
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
    /// Reads a file from the disk and adds it to the Lua library. It will not
    /// immediately be loaded. Logs an error if the file could not be read.
    ///
    /// If the filename conflicts with an existing one, then the existing file
    /// will be overwritten.
    pub fn read_file(&self, filename: String, file_path: &Path) {
        let file_path = file_path.strip_prefix(".").unwrap_or(file_path);
        match std::fs::read_to_string(file_path) {
            Ok(contents) => self.add_file(filename, contents),
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
                        let name = path
                            .strip_prefix(directory)
                            .unwrap_or(path)
                            .components()
                            .map(|component| component.as_os_str().to_string_lossy())
                            .join("/");
                        self.read_file(name, path);
                    }
                }
                Err(e) => log::warn!("error reading filesystem entry: {e:?}"),
            }
        }
    }
    /// Removes a file from the Lua library.
    pub fn remove_file(&self, filename: String) {
        self.send_command(LibraryCommand::RemoveFile { filename });
    }
    /// Unloads all files.
    pub fn remove_all_files(&self) {
        self.send_command(LibraryCommand::RemoveAllFiles);
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
