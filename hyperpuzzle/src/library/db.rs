use std::collections::{BTreeMap, HashMap};
use std::fmt;
use std::path::PathBuf;
use std::sync::Arc;

use eyre::{eyre, Result};
use mlua::prelude::*;
use parking_lot::Mutex;

use super::{LibraryFile, LibraryFileLoadOutput, LibraryFileLoadState, PuzzleRequestResponse};
use crate::builder::ColorSystemBuilder;
use crate::lua::PuzzleGeneratorOutput;
use crate::puzzle::Puzzle;

const MAX_PUZZLE_REDIRECTS: usize = 20;

/// Global library of shapes, puzzles, twist systems, etc.
#[derive(Default)]
pub(crate) struct LibraryDb {
    /// Map from filename to file.
    pub files: HashMap<String, Arc<LibraryFile>>,

    /// Map from the ID of a puzzle to the file in which it was defined.
    pub puzzles: BTreeMap<String, Arc<LibraryFile>>,
    /// Map from the ID of a puzzle to the file in which it was defined.
    pub puzzle_generators: BTreeMap<String, Arc<LibraryFile>>,
    /// Map from the ID of a color system to the file in which it was defined.
    pub color_systems: BTreeMap<String, Arc<LibraryFile>>,
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
    pub fn get(lua: &Lua) -> LuaResult<Arc<Mutex<LibraryDb>>> {
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
        let mut id = id.to_owned();
        let mut redirect_sequence = vec![id.clone()];

        for _ in 0..MAX_PUZZLE_REDIRECTS {
            let parsed_id = crate::parse_generated_puzzle_id(&id);

            let puzzle_request_response = match &parsed_id {
                Some((generator_id, _params)) => Self::get_file_result_for(
                    lua,
                    "puzzle generator",
                    generator_id,
                    |db| &db.puzzle_generators,
                    |file_output| file_output.request_puzzle(&id),
                )?,

                None => Self::get_file_result_for(
                    lua,
                    "puzzle",
                    &id,
                    |db| &db.puzzles,
                    |file_output| file_output.request_puzzle(&id),
                )?,
            };

            let (puzzle_spec, store_in_cache) = match puzzle_request_response {
                PuzzleRequestResponse::Generator {
                    generator,
                    generator_param_values,
                    store_in_cache,
                } => match generator.generate_puzzle_spec(lua, generator_param_values, None)? {
                    PuzzleGeneratorOutput::Puzzle(spec) => (spec, store_in_cache),
                    PuzzleGeneratorOutput::Redirect(new_id) => {
                        redirect_sequence.push(new_id.clone());
                        id = new_id;
                        continue;
                    }
                },

                PuzzleRequestResponse::Puzzle {
                    spec,
                    store_in_cache,
                } => (spec, store_in_cache),

                PuzzleRequestResponse::Cached(constructed_puzzle) => return Ok(constructed_puzzle),
            };

            let constructed_puzzle = puzzle_spec.build(lua)?;

            // Store constructed puzzle in cache.
            match parsed_id {
                Some((generator_id, _params)) => Self::get_file_result_for(
                    lua,
                    "puzzle generator",
                    generator_id,
                    |db| &db.puzzle_generators,
                    |file_output| store_in_cache(file_output, &constructed_puzzle),
                )?,

                None => Self::get_file_result_for(
                    lua,
                    "puzzle",
                    &id,
                    |db| &db.puzzles,
                    |file_output| store_in_cache(file_output, &constructed_puzzle),
                )?,
            }

            return Ok(constructed_puzzle);
        }

        Err(eyre!("too many puzzle redirects: {redirect_sequence:?}"))
    }
    /// Constructs the color system with ID `id`, or returns a previously cached
    /// result if it has already been constructed.
    ///
    /// Returns an error if an internal error occurred or if the user's code
    /// produced errors.
    pub fn build_color_system(lua: &Lua, id: &str) -> LuaResult<ColorSystemBuilder> {
        Self::get_file_result_for(
            lua,
            "color system",
            id,
            |db| &db.color_systems,
            |file_output| Some((**file_output.color_systems.get(id)?).clone()),
        )
    }

    fn get_file_result_for<O>(
        lua: &Lua,
        kind_of_thing: &str,
        id: &str,
        access_lib_db: impl FnOnce(&LibraryDb) -> &BTreeMap<String, Arc<LibraryFile>>,
        access_result: impl FnOnce(&mut LibraryFileLoadOutput) -> Option<O>,
    ) -> LuaResult<O> {
        let err_not_found = || LuaError::external(format!("no {kind_of_thing} with ID {id:?}"));
        let db = LibraryDb::get(lua)?;
        let db_guard = db.lock();
        let file = Arc::clone(
            access_lib_db(&*db_guard)
                .get(id)
                .ok_or_else(err_not_found)?,
        );
        drop(db_guard);
        let mut file_result = file.as_completed().ok_or_else(|| {
            LuaError::external(format!(
                "file {:?} owns {kind_of_thing} with ID {id:?} but is unloaded",
                file.name,
            ))
        })?;
        access_result(&mut *file_result).ok_or_else(err_not_found)
    }

    /// Adds a file to the Lua library. It will not immediately be loaded.
    ///
    /// If the filename conflicts with an existing one, then the existing file
    /// will be unloaded and overwritten.
    pub fn add_file(&mut self, filename: String, path: Option<PathBuf>, contents: String) {
        let file = LibraryFile {
            name: filename.clone(),
            path,
            contents,
            load_state: Mutex::new(LibraryFileLoadState::Unloaded),
            dependents: Mutex::new(vec![]),
        };

        // If the name, path, and contents are the same, then in theory we don't
        // need to reload the file. In practice, this is actually really
        // annoying and confusing when developing puzzles. If the user wants to
        // reload, just reload.

        // if let Some(existing_file) = self.files.get(&filename) {
        //     if **existing_file == file {
        //         //
        //         return;
        //     }
        // }

        self.unload_file(&filename);
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
            let LibraryFileLoadOutput {
                exports: _,

                puzzles,
                puzzle_generators,
                color_systems,
            } = result;

            for puzzle_id in puzzles.keys() {
                self.puzzles.remove(puzzle_id);
            }
            for puzzle_generator_id in puzzle_generators.keys() {
                self.puzzle_generators.remove(puzzle_generator_id);
            }
            for color_system_id in color_systems.keys() {
                self.color_systems.remove(color_system_id);
            }
        }
    }

    /// Unloads and removes a file from the Lua library.
    pub fn remove_file(&mut self, filename: &str) {
        self.unload_file(filename);
        self.files.remove(filename);
    }
}
