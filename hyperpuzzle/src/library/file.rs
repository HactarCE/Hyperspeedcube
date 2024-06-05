use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use mlua::prelude::*;
use parking_lot::{MappedMutexGuard, Mutex, MutexGuard};

use crate::lua::PuzzleParams;

use super::LazyPuzzle;

/// File stored in a [`super::Library`].
#[derive(Debug)]
pub struct LibraryFile {
    /// Name of the file. This may be chosen arbitrarily by the calling code,
    /// and may include some or all of the path.
    pub name: String,
    /// The path to the file. If specified, this may be used to reload the file
    /// if it changes.
    pub path: Option<PathBuf>,
    /// Contents of the file. This should be valid Lua code.
    pub contents: String,
    /// Load state of the file, which includes whether it has been loaded and
    /// the IDs of any library objects that were defined in it.
    pub(crate) load_state: Mutex<LibraryFileLoadState>,
    /// List of files that depend on this one. If this file is reloaded, those
    /// others must be reloaded as well.
    pub dependents: Mutex<Vec<Arc<LibraryFile>>>,
}
impl LibraryFile {
    /// Returns the file currently being loaded, given a Lua instance.
    pub(crate) fn get_current(lua: &Lua) -> LuaResult<Arc<LibraryFile>> {
        match lua.app_data_ref::<Arc<LibraryFile>>() {
            Some(file) => Ok(Arc::clone(&*file)),
            None => Err(LuaError::external(
                "this operation is only allowed while loading a file for the first time",
            )),
        }
    }

    /// Returns the in-progress result of the file, assuming it is the one
    /// currently being loaded.
    ///
    /// Returns an error if the file is not currently being loaded.
    pub(crate) fn as_loading(&self) -> LuaResult<MappedMutexGuard<'_, LibraryFileLoadResult>> {
        MutexGuard::try_map(self.load_state.lock(), |load_state| match load_state {
            LibraryFileLoadState::Loading(result) => Some(result),
            _ => None,
        })
        .map_err(|_| LuaError::external("current file is not in 'loading' state"))
    }

    /// Defines an object in the file.
    pub(crate) fn define_puzzle(&self, id: String, params: PuzzleParams) -> LuaResult<()> {
        match self
            .as_loading()?
            .puzzles
            .insert(id.clone(), LazyPuzzle::new(params))
        {
            Some(_old) => Err(LuaError::external(format!(
                "duplicate puzzle with ID {id:?}",
            ))),
            None => Ok(()),
        }
    }

    /// Returns whether the file is loaded.
    pub fn is_loaded(&self) -> bool {
        matches!(*self.load_state.lock(), LibraryFileLoadState::Done(_))
    }

    /// Returns the completed result of the file, assuming it has already been
    /// loaded.
    ///
    /// Returns `None` if the file has not yet been loaded.
    pub(crate) fn as_completed(&self) -> Option<MappedMutexGuard<'_, LibraryFileLoadResult>> {
        MutexGuard::try_map(self.load_state.lock(), |load_state| {
            load_state.completed_mut()
        })
        .ok()
    }
}

/// Load state and data for a [`LibraryFile`].
#[derive(Debug, Default)]
pub(crate) enum LibraryFileLoadState {
    /// The file has not yet been loaded.
    #[default]
    Unloaded,
    /// The file is currently being loaded.
    Loading(LibraryFileLoadResult),
    /// The file has been loaded.
    Done(LuaResult<LibraryFileLoadResult>),
}
impl LibraryFileLoadState {
    /// Finish loading the file successfully.
    ///
    /// Returns an error if the file is not currently being loaded.
    pub(crate) fn complete_ok<'lua>(&mut self, lua: &'lua Lua) -> LuaResult<LuaTable<'lua>> {
        match std::mem::take(self) {
            LibraryFileLoadState::Loading(load_result) => {
                let exports_table = lua.registry_value(&load_result.exports);
                *self = LibraryFileLoadState::Done(Ok(load_result));
                exports_table
            }
            _ => {
                let err = LuaError::external(format!("bad load state: {self:?}"));
                Err(self.complete_err(err.clone()))
            }
        }
    }
    /// Finish loading the file unsuccessfully. The error `e` is recorded for
    /// the file and then returned.
    pub fn complete_err(&mut self, e: LuaError) -> LuaError {
        *self = LibraryFileLoadState::Done(Err(e.clone()));
        e
    }
    /// Returns a mutable reference to the completed load result for the file.
    pub fn completed_mut(&mut self) -> Option<&mut LibraryFileLoadResult> {
        match self {
            LibraryFileLoadState::Done(result) => result.as_mut().ok(),
            _ => None,
        }
    }
}

/// Data from loading a [`LibraryFile`].
#[derive(Debug)]
pub(crate) struct LibraryFileLoadResult {
    /// Table of exports to other Lua code that imports this file.
    pub exports: LuaRegistryKey,
    /// Puzzles defined in this file, indexed by ID.
    pub puzzles: HashMap<String, LazyPuzzle>,
}
impl LibraryFileLoadResult {
    /// Constructs an empty load result.
    pub(crate) fn with_exports(exports_table: LuaRegistryKey) -> Self {
        Self {
            exports: exports_table,
            puzzles: HashMap::new(),
        }
    }
}
