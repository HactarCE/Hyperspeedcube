use std::collections::hash_map;
use std::path::{Path, PathBuf};
use std::sync::{mpsc, Arc};

use itertools::Itertools;
use parking_lot::Mutex;

use super::{
    LibraryCommand, LibraryDb, NotifyWhenDropped, PuzzleBuildStatus, PuzzleCacheEntry, Waiter,
};
use crate::builder::ColorSystemBuilder;
use crate::lua::{LuaLoader, LuaLogger, PuzzleGeneratorSpec, PuzzleSpec};
use crate::{LuaLogLine, Puzzle};

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
        let loader =
            LuaLoader::new(Arc::clone(&db), logger).expect("error initializing Lua environment");

        std::thread::spawn(move || {
            for command in cmd_rx {
                match command {
                    LibraryCommand::Reset => {
                        loader.db.lock().reset();
                    }
                    LibraryCommand::ReadDirectory { directory } => {
                        loader.db.lock().read_directory(&directory);
                    }
                    LibraryCommand::AddFile {
                        filename,
                        path,
                        contents,
                    } => {
                        loader.db.lock().add_file(filename, path, contents);
                    }
                    LibraryCommand::LoadFiles => {
                        loader.load_all_files();
                    }
                    LibraryCommand::BuildPuzzle { id } => {
                        loader.build_puzzle(&id);
                    }
                    LibraryCommand::Wait(sender) => {
                        let _ = sender.send(());
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
        self.send_command(LibraryCommand::AddFile {
            filename,
            path,
            contents,
        });
    }
    /// Reads a directory recursively and adds all files ending in `.lua` to the
    /// Lua library. They will not immediately be loaded.
    ///
    /// If any filename conflicts with an existing one, then the existing file
    /// will be overwritten.
    pub fn read_directory(&self, directory: &Path) {
        self.send_command(LibraryCommand::ReadDirectory {
            directory: directory.to_owned(),
        });
    }
    /// Canonicalizes a relative file path to make a suitable filename.
    pub fn relative_path_to_filename(path: &Path) -> String {
        path.components()
            .map(|component| component.as_os_str().to_string_lossy())
            .join("/")
    }
    /// Loads all files that haven't been loaded yet. Lua execution happens
    /// asynchronously, so changes might not take effect immediately.
    pub fn load_files(&self) {
        self.send_command(LibraryCommand::LoadFiles);
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
    /// Returns a list of puzzle authors.
    pub fn authors(&self) -> Vec<String> {
        self.db.lock().authors()
    }

    /// Builds a puzzle from a Lua specification.
    pub fn build_puzzle(&self, id: &str) -> PuzzleResult {
        let mut id = id.to_owned();
        for _ in 0..crate::MAX_PUZZLE_REDIRECTS {
            return match self.db.lock().puzzle_cache.entry(id.to_string()) {
                hash_map::Entry::Vacant(e) => {
                    let notify = NotifyWhenDropped::new();
                    let waiter = notify.waiter();
                    let status = None;
                    e.insert(Arc::new(Mutex::new(PuzzleCacheEntry::Building {
                        notify,
                        status,
                    })));
                    self.send_command(LibraryCommand::BuildPuzzle { id: id.to_string() });
                    let status = None;
                    PuzzleResult::Building { waiter, status }
                }

                hash_map::Entry::Occupied(e) => match &*e.get().lock() {
                    PuzzleCacheEntry::Redirect(new_id) => {
                        id = new_id.clone();
                        continue;
                    }
                    PuzzleCacheEntry::Building { notify, status } => PuzzleResult::Building {
                        waiter: notify.waiter(),
                        status: status.clone(),
                    },
                    PuzzleCacheEntry::Ok(puzzle) => PuzzleResult::Ok(Arc::clone(puzzle)),
                    PuzzleCacheEntry::Err => PuzzleResult::Err,
                },
            };
        }
        PuzzleResult::Err // too many redirects
    }

    pub fn build_puzzle_blocking(&self, id: &str) -> Result<Arc<Puzzle>, ()> {
        loop {
            match self.build_puzzle(id) {
                PuzzleResult::Ok(puzzle) => return Ok(puzzle),
                PuzzleResult::Building { waiter, .. } => waiter.wait(),
                PuzzleResult::Err => return Err(()),
            }
        }
    }

    /// Creates a new library with a fresh Lua state.
    pub fn reset(&self) {
        self.send_command(LibraryCommand::Reset);
    }

    /// Waits until all pending library tasks are completed.
    ///
    /// **This method is blocking.**
    pub fn wait(&self) {
        let (tx, rx) = mpsc::sync_channel(0);
        self.send_command(LibraryCommand::Wait(tx));
        let _ = rx.recv();
    }
}

pub enum PuzzleResult {
    Ok(Arc<Puzzle>),
    Building {
        waiter: Waiter,
        status: Option<PuzzleBuildStatus>,
    },
    Err,
}
