use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use mlua::prelude::*;
use parking_lot::{MappedMutexGuard, Mutex, MutexGuard};

use super::{LazyPuzzle, LazyPuzzleGenerator};
use crate::{
    builder::ColorSystemBuilder,
    lua::{PuzzleGeneratorSpec, PuzzleSpec},
    Puzzle,
};

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
impl PartialEq for LibraryFile {
    fn eq(&self, other: &Self) -> bool {
        // Ignore load state and dependents when comparing files.
        self.name == other.name && self.path == other.path && self.contents == other.contents
    }
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
    pub(crate) fn as_loading(&self) -> LuaResult<MappedMutexGuard<'_, LibraryFileLoadOutput>> {
        MutexGuard::try_map(self.load_state.lock(), |load_state| match load_state {
            LibraryFileLoadState::Loading(result) => Some(result),
            _ => None,
        })
        .map_err(|_| LuaError::external("current file is not in 'loading' state"))
    }

    /// Defines a puzzle in the file.
    pub(crate) fn define_puzzle(&self, spec: PuzzleSpec) -> LuaResult<()> {
        self.define_object(
            |file_output| &mut file_output.puzzles,
            spec.id.clone(),
            LazyPuzzle::new(spec),
            "puzzle",
        )
    }
    /// Defines a puzzle generator in the file.
    pub(crate) fn define_puzzle_generator(&self, generator: PuzzleGeneratorSpec) -> LuaResult<()> {
        self.define_object(
            |file_output| &mut file_output.puzzle_generators,
            generator.id.clone(),
            LazyPuzzleGenerator::new(generator),
            "puzzle generator",
        )
    }
    /// Defines a color system in the file.
    pub(crate) fn define_color_system(&self, color_system: ColorSystemBuilder) -> LuaResult<()> {
        self.define_object(
            |file_output| &mut file_output.color_systems,
            color_system.id.clone(),
            Arc::new(color_system),
            "color system",
        )
    }

    fn define_object<O>(
        &self,
        get_hashmap: impl FnOnce(&mut LibraryFileLoadOutput) -> &mut HashMap<String, O>,
        id: String,
        obj: O,
        obj_type_str: &str,
    ) -> LuaResult<()> {
        match get_hashmap(&mut *self.as_loading()?).insert(id.clone(), obj) {
            Some(_old) => Err(LuaError::external(format!(
                "duplicate {obj_type_str} with ID {id:?}",
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
    pub(crate) fn as_completed(&self) -> Option<MappedMutexGuard<'_, LibraryFileLoadOutput>> {
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
    Loading(LibraryFileLoadOutput),
    /// The file has been loaded.
    Done(LuaResult<LibraryFileLoadOutput>),
}
impl LibraryFileLoadState {
    /// Finish loading the file successfully.
    ///
    /// Returns an error if the file is not currently being loaded.
    pub(crate) fn complete_ok(&mut self, lua: &Lua) -> LuaResult<LuaTable> {
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
    pub fn completed_mut(&mut self) -> Option<&mut LibraryFileLoadOutput> {
        match self {
            LibraryFileLoadState::Done(result) => result.as_mut().ok(),
            _ => None,
        }
    }
}

/// Data from loading a [`LibraryFile`].
#[derive(Debug)]
pub(crate) struct LibraryFileLoadOutput {
    /// Table of exports to other Lua code that imports this file.
    pub exports: LuaRegistryKey,
    /// Puzzles defined in this file, indexed by ID.
    pub puzzles: HashMap<String, LazyPuzzle>,
    /// Puzzle generators defined in this file, indexed by ID.
    pub puzzle_generators: HashMap<String, LazyPuzzleGenerator>,
    /// Color systems defined in this file, indexed by ID.
    pub color_systems: HashMap<String, Arc<ColorSystemBuilder>>,
}
impl LibraryFileLoadOutput {
    /// Constructs an empty load result.
    pub(crate) fn with_exports(exports_table: LuaRegistryKey) -> Self {
        Self {
            exports: exports_table,
            puzzles: HashMap::new(),
            puzzle_generators: HashMap::new(),
            color_systems: HashMap::new(),
        }
    }

    pub(super) fn request_puzzle(&self, puzzle_id: &str) -> Option<PuzzleRequestResponse> {
        match crate::parse_generated_puzzle_id(puzzle_id) {
            Some((generator_id, params)) => {
                let cache = self.puzzle_generators.get(generator_id)?;
                match cache.constructed.get(puzzle_id) {
                    Some(constructed) => {
                        Some(PuzzleRequestResponse::Cached(Arc::clone(constructed)))
                    }
                    None => {
                        let param_values = params.iter().map(|val| val.to_string()).collect();
                        let generator_id = generator_id.to_owned();
                        let puzzle_id = puzzle_id.to_owned();
                        Some(PuzzleRequestResponse::Generator {
                            generator: Arc::clone(&cache.generator),
                            generator_param_values: param_values,
                            store_in_cache: Box::new(move |this, puzzle| {
                                this.puzzle_generators
                                    .get_mut(&generator_id)?
                                    .constructed
                                    .insert(puzzle_id, Arc::clone(puzzle));
                                Some(())
                            }),
                        })
                    }
                }
            }

            None => {
                let cache = self.puzzles.get(puzzle_id)?;
                match &cache.constructed {
                    Some(constructed) => {
                        Some(PuzzleRequestResponse::Cached(Arc::clone(constructed)))
                    }
                    None => {
                        let puzzle_id = puzzle_id.to_owned();
                        Some(PuzzleRequestResponse::Puzzle {
                            spec: Arc::clone(&cache.spec),
                            store_in_cache: Box::new(move |file_output, puzzle| {
                                file_output.puzzles.get_mut(&puzzle_id)?.constructed =
                                    Some(Arc::clone(puzzle));
                                Some(())
                            }),
                        })
                    }
                }
            }
        }
    }
}

pub(super) enum PuzzleRequestResponse {
    Generator {
        generator: Arc<PuzzleGeneratorSpec>,
        generator_param_values: Vec<String>,
        store_in_cache: Box<dyn FnOnce(&mut LibraryFileLoadOutput, &Arc<Puzzle>) -> Option<()>>,
    },
    Puzzle {
        spec: Arc<PuzzleSpec>,
        store_in_cache: Box<dyn FnOnce(&mut LibraryFileLoadOutput, &Arc<Puzzle>) -> Option<()>>,
    },
    Cached(Arc<Puzzle>),
}
