use std::collections::{BTreeMap, HashMap};
use std::fmt;
use std::path::PathBuf;
use std::sync::Arc;

use eyre::Result;
use mlua::prelude::*;
use parking_lot::Mutex;

use super::{LibraryFile, LibraryFileLoadResult, LibraryFileLoadState};
use crate::puzzle::Puzzle;

/// Global library of shapes, puzzles, twist systems, etc.
#[derive(Default)]
pub(crate) struct LibraryDb {
    /// Map from filename to file.
    pub files: HashMap<String, Arc<LibraryFile>>,

    /// Map from the name of a puzzle to the file in which it was defined.
    pub puzzles: BTreeMap<String, Arc<LibraryFile>>,
}
impl fmt::Debug for LibraryDb {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_set().entries(self.files.keys()).finish()
    }
}
impl LibraryDb {
    /// Constructs a new library.
    pub fn new() -> Arc<Mutex<Self>> {
        Arc::new(Mutex::new(Self::default()))
    }
    /// Returns the global library, given a Lua instance.
    pub fn get<'lua>(lua: &'lua Lua) -> LuaResult<Arc<Mutex<LibraryDb>>> {
        Ok(Arc::clone(
            &*lua
                .app_data_ref::<Arc<Mutex<LibraryDb>>>()
                .ok_or_else(|| LuaError::external("no library"))?,
        ))
    }

    /// Constructs the puzzle with ID `id`, or returns a previously cached
    /// result if it has already been constructed.
    ///
    /// Returns an error if an internal error occurred or if the user's code
    /// produced errors.
    pub fn build_puzzle(lua: &Lua, id: &str) -> Result<Arc<Puzzle>> {
        let err_not_found = || LuaError::external(format!("no puzzle with ID {id:?}"));
        let db = LibraryDb::get(lua)?;
        let db_guard = db.lock();
        let file = db_guard.puzzles.get(id).ok_or_else(err_not_found)?;
        let mut file_result = file.as_completed().ok_or_else(|| {
            LuaError::external(format!(
                "file {:?} owns puzzle with ID {id:?} but is unloaded",
                file.name,
            ))
        })?;
        let cache = file_result.puzzles.get_mut(id).ok_or_else(err_not_found)?;
        if let Some(constructed) = &cache.constructed {
            return Ok(Arc::clone(&constructed));
        }
        let constructed_puzzle = cache.params.build(lua)?;
        cache.constructed = Some(Arc::clone(&constructed_puzzle));
        Ok(constructed_puzzle)
    }

    /// Adds a file to the Lua library. It will not immediately be loaded.
    ///
    /// If the filename conflicts with an existing one, then the existing file
    /// will be unloaded and overwritten.
    pub fn add_file(&mut self, filename: String, path: Option<PathBuf>, contents: String) {
        self.unload_file(&filename);
        let file = LibraryFile {
            name: filename.clone(),
            path,
            contents,
            load_state: Mutex::new(LibraryFileLoadState::Unloaded),
            dependents: Mutex::new(vec![]),
        };
        self.files.insert(filename, Arc::new(file));
    }

    /// Unloads a file.
    pub fn unload_file(&mut self, filename: &str) {
        // If the file doesn't exist, don't worry about it.
        let Some(file) = self.files.get_mut(filename) else {
            return;
        };

        let dependents = std::mem::take(&mut *file.dependents.lock());
        let load_state = std::mem::take(&mut *file.load_state.lock());

        for dep in dependents {
            self.unload_file(&dep.name);
        }

        if let LibraryFileLoadState::Done(Ok(result)) = load_state {
            let LibraryFileLoadResult {
                exports: _,

                puzzles,
            } = result;

            for puzzle_id in puzzles.keys() {
                self.puzzles.remove(puzzle_id);
            }
        }
    }

    /// Unloads and removes a file from the Lua library.
    pub fn remove_file(&mut self, filename: &str) {
        self.unload_file(filename);
        self.files.remove(filename);
    }
}
