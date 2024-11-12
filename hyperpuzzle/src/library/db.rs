use std::collections::{BTreeMap, HashMap};
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use eyre::{eyre, Result};
use mlua::prelude::*;
use parking_lot::Mutex;

use super::{Library, LibraryFile, LibraryFileLoadState};
use crate::builder::ColorSystemBuilder;
use crate::lua::{PuzzleGeneratorOutput, PuzzleGeneratorSpec, PuzzleSpec};
use crate::puzzle::Puzzle;

const MAX_PUZZLE_REDIRECTS: usize = 20;

/// Global library of shapes, puzzles, twist systems, etc.
#[derive(Default)]
pub(crate) struct LibraryDb {
    /// File contents by file path, only for unloaded files.
    pub files: HashMap<String, LibraryFile>,

    /// Loaded puzzles by ID.
    pub puzzles: BTreeMap<String, Arc<PuzzleSpec>>,
    /// Loaded puzzle generators by ID.
    pub puzzle_generators: BTreeMap<String, Arc<PuzzleGeneratorSpec>>,
    /// Loaded color systems by ID.
    pub color_systems: BTreeMap<String, Arc<ColorSystemBuilder>>,

    /// Cache of constructed puzzles.
    pub puzzle_cache: HashMap<String, Arc<Puzzle>>,
}
impl fmt::Debug for LibraryDb {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LibraryDb")
            .field("files", &self.files.keys())
            .field("puzzles", &self.puzzles.keys())
            .field("puzzle_generators", &self.puzzle_generators.keys())
            .field("color_systems", &self.color_systems.keys())
            .finish()
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
            let db = LibraryDb::get(lua)?;
            let db_guard = db.lock();

            // Return cached puzzle if it was already constructed.
            if let Some(cached) = db_guard.puzzle_cache.get(&id) {
                return Ok(Arc::clone(cached));
            }

            let puzzle_spec = match crate::parse_generated_puzzle_id(&id) {
                Some((generator_id, params)) => {
                    let generator = db_guard
                        .puzzle_generators
                        .get(generator_id)
                        .ok_or_else(|| eyre!("no puzzle generator with ID {generator_id:?}"))?;
                    let generator_param_values = params.into_iter().map(str::to_owned).collect();
                    match generator.generate_puzzle_spec(lua, generator_param_values, None)? {
                        PuzzleGeneratorOutput::Puzzle(spec) => spec,
                        PuzzleGeneratorOutput::Redirect(new_id) => {
                            redirect_sequence.push(new_id.clone());
                            id = new_id;
                            continue;
                        }
                    }
                }
                None => Arc::clone(
                    db_guard
                        .puzzles
                        .get(&id)
                        .ok_or_else(|| eyre!("no puzzle with ID {id:?}"))?,
                ),
            };

            drop(db_guard);

            let constructed_puzzle = puzzle_spec.build(lua)?;

            db.lock()
                .puzzle_cache
                .insert(id, Arc::clone(&constructed_puzzle));

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
        let err = || LuaError::external(format!("no color system with ID {id:?}"));
        Ok((**LibraryDb::get(lua)?
            .lock()
            .color_systems
            .get(id)
            .ok_or_else(err)?)
        .clone())
    }

    /// Adds a file to the Lua library.
    ///
    /// See [`crate::Library::add_file()`].
    pub fn add_file(&mut self, filename: String, path: Option<PathBuf>, contents: String) {
        self.files.insert(
            filename.clone(),
            LibraryFile {
                name: filename,
                path,
                contents,

                load_state: LibraryFileLoadState::Unloaded,
            },
        );
    }

    /// Reads a file from the disk and adds it to the Lua library.
    ///
    /// See [`crate::Library::read_file()`].
    pub fn read_file(&mut self, filename: String, path: PathBuf) {
        let file_path = path.strip_prefix(".").unwrap_or(&path);
        match std::fs::read_to_string(file_path) {
            Ok(contents) => self.add_file(filename, Some(file_path.to_path_buf()), contents),
            Err(e) => log::error!("error loading {file_path:?}: {e}"),
        }
    }

    pub fn read_directory(&mut self, directory: &Path) {
        for entry in walkdir::WalkDir::new(directory).follow_links(true) {
            match entry {
                Ok(entry) => {
                    let path = entry.path();
                    if path.extension().is_some_and(|ext| ext == "lua") {
                        let relative_path = path.strip_prefix(directory).unwrap_or(path);
                        let name = Library::relative_path_to_filename(relative_path);
                        self.read_file(name, path.to_owned());
                    }
                }
                Err(e) => log::warn!("error reading filesystem entry: {e:?}"),
            }
        }
    }
}
