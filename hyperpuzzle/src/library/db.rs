use eyre::Result;
use std::collections::{BTreeMap, HashMap};
use std::fmt;
use std::path::PathBuf;
use std::sync::Arc;

use hypershape::Space;
use mlua::prelude::*;
use parking_lot::Mutex;

use crate::lua::{LuaNdim, NilStringOrRegisteredTable, PuzzleParams};
use crate::puzzle::Puzzle;

use super::{
    Cached, CachedPuzzle, LibraryFile, LibraryFileLoadResult, LibraryFileLoadState,
    LibraryObjectParams,
};

/// Global library of shapes, puzzles, twist systems, etc.
#[derive(Default)]
pub struct LibraryDb {
    pub files: HashMap<String, Arc<LibraryFile>>,

    pub shapes: BTreeMap<String, Arc<LibraryFile>>,
    pub axis_systems: BTreeMap<String, Arc<LibraryFile>>,
    pub twist_systems: BTreeMap<String, Arc<LibraryFile>>,
    pub puzzles: BTreeMap<String, Arc<LibraryFile>>,
}
impl fmt::Debug for LibraryDb {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_set().entries(self.files.keys()).finish()
    }
}
impl LibraryDb {
    pub fn new() -> Arc<Mutex<Self>> {
        Arc::new(Mutex::new(Self::default()))
    }
    pub fn get<'lua>(lua: &'lua Lua) -> LuaResult<Arc<Mutex<LibraryDb>>> {
        Ok(Arc::clone(
            &*lua
                .app_data_ref::<Arc<Mutex<LibraryDb>>>()
                .ok_or_else(|| LuaError::external("no library"))?,
        ))
    }

    fn with_object<P: LibraryObjectParams, R>(
        lua: &Lua,
        id: &str,
        f: impl FnOnce(&Cached<P>) -> LuaResult<R>,
    ) -> LuaResult<R> {
        let err = || LuaError::external(format!("no puzzle with ID {id:?}"));
        let db = LibraryDb::get(lua)?;
        let db_guard = db.lock();
        let file = P::get_file_map(&db_guard).get(id).ok_or_else(err)?;
        let mut result = file.as_completed().ok_or_else(|| {
            LuaError::external(format!(
                "file {:?} owns {} with ID {id:?} but is unloaded",
                file.name,
                P::NAME,
            ))
        })?;
        let cached = P::get_id_map_within_file(&mut *result)
            .get(id)
            .ok_or_else(err)?;
        f(cached)
    }

    pub(crate) fn build_from_id<P: LibraryObjectParams>(
        lua: &Lua,
        space: &Arc<Mutex<Space>>,
        id: &str,
    ) -> LuaResult<P::Constructed> {
        enum Lazy<P: LibraryObjectParams> {
            Constructed(P::Constructed),
            Unconstrurcted(Arc<P>),
        }

        let constructed = Self::with_object::<P, _>(lua, id, |cached| match &cached.constructed {
            Some(builder) => match P::clone_constructed(builder, &space) {
                Ok(constructed) => Ok(Lazy::Constructed(constructed)),
                Err(e) => Err(LuaError::external(e)),
            },
            None => Ok(Lazy::Unconstrurcted(Arc::clone(&cached.params))),
        })?;

        match constructed {
            Lazy::Constructed(builder) => Ok(builder),
            Lazy::Unconstrurcted(params) => params.build(lua, &space),
        }
    }

    pub(crate) fn build_from_value<P: LibraryObjectParams>(
        lua: &Lua,
        space: &Arc<Mutex<Space>>,
        id_or_table: &NilStringOrRegisteredTable,
    ) -> LuaResult<P::Constructed> {
        match id_or_table {
            // Build a default empty object.
            NilStringOrRegisteredTable::Nil => P::new_constructed(space),
            // Use an existing object.
            NilStringOrRegisteredTable::String(id) => Self::build_from_id::<P>(lua, space, &id),
            // Build a bespoke object just for this.
            NilStringOrRegisteredTable::Table(key) => {
                P::from_lua(lua.registry_value(key)?, lua)?.build(lua, space)
            }
        }
    }

    pub fn build_puzzle(lua: &Lua, id: &str) -> Result<Arc<Puzzle>> {
        let LuaNdim(ndim) =
            Self::with_object(lua, id, |cached: &CachedPuzzle| Ok(cached.params.ndim))?;
        let space = Arc::new(Mutex::new(Space::new(ndim)?));
        Ok(Self::build_from_id::<PuzzleParams>(lua, &space, id)?)
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
                dependencies: _,

                exports: _,

                shapes,
                axis_systems,
                twist_systems,
                puzzles,
            } = result;

            for shape_id in shapes.keys() {
                self.shapes.remove(shape_id);
            }
            for axis_system_id in axis_systems.keys() {
                self.axis_systems.remove(axis_system_id);
            }
            for twist_system_id in twist_systems.keys() {
                self.twist_systems.remove(twist_system_id);
            }
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
