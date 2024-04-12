use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use mlua::prelude::*;
use parking_lot::{MappedMutexGuard, Mutex, MutexGuard};

use super::{
    Cached, CachedAxisSystem, CachedPuzzle, CachedShape, CachedTwistSystem, LibraryObjectParams,
};

#[derive(Debug)]
pub struct LibraryFile {
    pub name: String,
    pub path: Option<PathBuf>,
    pub contents: String,
    pub load_state: Mutex<LibraryFileLoadState>,
    pub dependents: Mutex<Vec<Arc<LibraryFile>>>,
}
impl LibraryFile {
    pub fn new(name: String, path: Option<PathBuf>, contents: String) -> Self {
        Self {
            name,
            path,
            contents,
            load_state: Mutex::new(LibraryFileLoadState::Unloaded),
            dependents: Mutex::new(vec![]),
        }
    }

    pub fn get_current(lua: &Lua) -> LuaResult<Arc<LibraryFile>> {
        match lua.app_data_ref::<Arc<LibraryFile>>() {
            Some(file) => Ok(Arc::clone(&*file)),
            None => Err(LuaError::external(
                "this operation is only allowed while loading a file for the first time",
            )),
        }
    }
    pub fn as_loading(&self) -> LuaResult<MappedMutexGuard<'_, LibraryFileLoadResult>> {
        MutexGuard::try_map(self.load_state.lock(), |load_state| match load_state {
            LibraryFileLoadState::Loading(result) => Some(result),
            _ => None,
        })
        .map_err(|_| LuaError::external("current file is not in 'loading' state"))
    }

    pub fn insert<P: LibraryObjectParams>(&self, id: String, params: P) -> LuaResult<()> {
        match P::get_id_map_within_file(&mut *self.as_loading()?)
            .insert(id.clone(), Cached::new(params))
        {
            Some(_old) => Err(LuaError::external(format!(
                "duplicate {} with ID {id:?}",
                P::NAME
            ))),
            None => Ok(()),
        }
    }

    pub fn is_loaded(&self) -> bool {
        matches!(*self.load_state.lock(), LibraryFileLoadState::Done(_))
    }

    pub fn as_completed(&self) -> Option<MappedMutexGuard<'_, LibraryFileLoadResult>> {
        MutexGuard::try_map(self.load_state.lock(), |load_state| {
            load_state.completed_mut()
        })
        .ok()
    }
}

#[derive(Debug, Default)]
pub enum LibraryFileLoadState {
    #[default]
    Unloaded,
    Loading(LibraryFileLoadResult),
    Done(LuaResult<LibraryFileLoadResult>),
}
impl LibraryFileLoadState {
    pub fn complete_ok<'lua>(&mut self, lua: &'lua Lua) -> LuaResult<LuaTable<'lua>> {
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
    pub fn complete_err(&mut self, e: LuaError) -> LuaError {
        *self = LibraryFileLoadState::Done(Err(e.clone()));
        e
    }
    pub fn completed_ref(&mut self) -> Option<&LibraryFileLoadResult> {
        match self {
            LibraryFileLoadState::Done(result) => result.as_ref().ok(),
            _ => None,
        }
    }
    pub fn completed_mut(&mut self) -> Option<&mut LibraryFileLoadResult> {
        match self {
            LibraryFileLoadState::Done(result) => result.as_mut().ok(),
            _ => None,
        }
    }
}

#[derive(Debug)]
pub struct LibraryFileLoadResult {
    pub dependencies: Vec<Arc<LibraryFile>>,

    pub exports: LuaRegistryKey,

    pub shapes: HashMap<String, CachedShape>,
    pub axis_systems: HashMap<String, CachedAxisSystem>,
    pub twist_systems: HashMap<String, CachedTwistSystem>,
    pub puzzles: HashMap<String, CachedPuzzle>,
}
impl LibraryFileLoadResult {
    pub fn with_exports(exports_table: LuaRegistryKey) -> Self {
        Self {
            dependencies: vec![],

            exports: exports_table,

            shapes: HashMap::new(),
            axis_systems: HashMap::new(),
            twist_systems: HashMap::new(),
            puzzles: HashMap::new(),
        }
    }
}
