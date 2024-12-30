use std::collections::{hash_map, BTreeMap, HashMap, HashSet};
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use itertools::Itertools;
use mlua::prelude::*;
use parking_lot::{Condvar, Mutex};

use super::{Library, LibraryFile, LibraryFileLoadState};
use crate::builder::ColorSystemBuilder;
use crate::lua::{LuaLogger, PuzzleGeneratorOutput, PuzzleGeneratorSpec, PuzzleSpec};
use crate::puzzle::Puzzle;

/// Global library of shapes, puzzles, twist systems, etc.
#[derive(Default)]
pub(crate) struct LibraryDb {
    /// File contents by file path, only for unloaded files.
    pub files: HashMap<String, LibraryFile>,
    /// Set of directories that contain files.
    pub directories: HashSet<String>,

    /// Loaded puzzles by ID.
    pub puzzles: BTreeMap<String, Arc<PuzzleSpec>>,
    /// Loaded puzzle generators by ID.
    pub puzzle_generators: BTreeMap<String, Arc<PuzzleGeneratorSpec>>,
    /// Loaded color systems by ID.
    pub color_systems: BTreeMap<String, Arc<ColorSystemBuilder>>,

    /// Cache of constructed puzzles.
    pub puzzle_cache: HashMap<String, Arc<Mutex<PuzzleCacheEntry>>>,
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
    pub fn get(lua: &Lua) -> Arc<Mutex<LibraryDb>> {
        Arc::clone(
            &*lua
                .app_data_ref::<Arc<Mutex<LibraryDb>>>()
                .expect("no Lua library"),
        )
    }

    /// Deletes everything from the library except for the files and their contents.
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// Constructs the puzzle with ID `id` if it has not already been
    /// constructed.
    ///
    /// Returns an error if an internal error occurred or if the user's code
    /// produced errors.
    pub fn build_puzzle(lua: &Lua, logger: &LuaLogger, id: &str) {
        let mut id = id.to_owned();
        let mut redirect_sequence = vec![];
        let db = LibraryDb::get(lua);

        enum BuildPuzzleResult {
            Old(Arc<Mutex<Arc<Puzzle>>>),
            New(Arc<Puzzle>),
        }

        for _ in 0..crate::MAX_PUZZLE_REDIRECTS {
            redirect_sequence.push(id.clone());

            let mut db_guard = db.lock();

            let mut file = None;

            let cache_entry = Arc::clone(db_guard.puzzle_cache.entry(id.clone()).or_default());
            let mut cache_entry_guard = cache_entry.lock();
            match &mut *cache_entry_guard {
                // The puzzle has an ID redirect.
                PuzzleCacheEntry::Redirect(new_id) => id = new_id.clone(),

                // The puzzle was requested but has not started being built.
                PuzzleCacheEntry::Building { notify: _, status } if status.is_none() => {
                    // Mark that this puzzle is being built.
                    *status = Some(PuzzleBuildStatus {});
                    // Unlock the mutex before running Lua code.
                    drop(cache_entry_guard);
                    // Get the puzzle spec, which may involve running Lua code.
                    let generator_output = match crate::parse_generated_puzzle_id(&id) {
                        None => match db_guard.puzzles.get(&id).cloned() {
                            None => Err(format!("no puzzle with ID {id:?}")),
                            Some(spec) => {
                                drop(db_guard);
                                file = Some(spec.tags.filename().to_owned());
                                Ok(PuzzleGeneratorOutput::Puzzle(spec))
                            }
                        },
                        Some((generator_id, params)) => {
                            match db_guard.puzzle_generators.get(generator_id).cloned() {
                                None => Err(format!("no generator with ID {generator_id:?}")),
                                Some(generator) => {
                                    drop(db_guard); // unlock mutex before running Lua code
                                    file = Some(generator.tags.filename().to_owned());
                                    let generator_param_values =
                                        params.into_iter().map(str::to_owned).collect();
                                    generator
                                        .generate_puzzle_spec(lua, generator_param_values, None)
                                        .map_err(|e| format!("{e:#}"))
                                }
                            }
                        }
                    };
                    // Build the puzzle, which will certainly run Lua code.
                    let mut redirect_id = None;
                    let puzzle_cache_entry_value = generator_output
                        .and_then(|output| match output {
                            PuzzleGeneratorOutput::Puzzle(puzzle_spec) => {
                                match puzzle_spec.build(lua) {
                                    Ok(puzzle) => Ok(PuzzleCacheEntry::Ok(puzzle)),
                                    Err(e) => Err(format!("{e:#}")),
                                }
                            }
                            PuzzleGeneratorOutput::Redirect(new_id) => {
                                redirect_id = Some(new_id.clone());
                                Ok(PuzzleCacheEntry::Redirect(new_id))
                            }
                        })
                        .unwrap_or_else(|e| {
                            let msg = format!("error building puzzle {id}: {e:#}");
                            logger.error(file.clone(), msg);
                            PuzzleCacheEntry::Err
                        });
                    *cache_entry.lock() = puzzle_cache_entry_value;
                    if let Some(new_id) = redirect_id {
                        id = new_id;
                    } else {
                        return;
                    }
                }

                // The puzzle has already been built or is being built.
                _ => return,
            }
        }

        let msg = format!("too many puzzle redirects: {redirect_sequence:?}");
        logger.error(None, msg);
    }
    /// Constructs the color system with ID `id`, or returns a previously cached
    /// result if it has already been constructed.
    ///
    /// Returns an error if an internal error occurred or if the user's code
    /// produced errors.
    pub fn build_color_system(lua: &Lua, id: &str) -> LuaResult<ColorSystemBuilder> {
        let err = || LuaError::external(format!("no color system with ID {id:?}"));
        Ok((**LibraryDb::get(lua)
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
        let mut dirname = filename.as_str();
        while let Some((prefix, _)) = dirname.rsplit_once('/') {
            dirname = prefix;
            self.directories.insert(dirname.to_string());
        }
        self.files.insert(
            filename.clone(),
            LibraryFile {
                name: filename,
                path,
                contents: Some(contents),

                load_state: LibraryFileLoadState::Unloaded,
            },
        );
    }

    /// Reads a file from the disk and adds it to the Lua library using
    /// [`Self::add_file()`].
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

    pub fn authors(&self) -> Vec<String> {
        // TODO: cache this when loading files
        itertools::chain(
            self.puzzles.values().flat_map(|p| p.tags.authors()),
            self.puzzle_generators
                .values()
                .flat_map(|g| g.tags.authors()),
        )
        .unique()
        .map(|s| s.to_string())
        .sorted()
        .collect_vec()
    }
}

#[derive(Debug)]
pub enum PuzzleCacheEntry {
    Redirect(String),
    Building {
        notify: NotifyWhenDropped,
        /// If this is `None`, then the puzzle has not started being built.
        status: Option<PuzzleBuildStatus>,
    },
    Ok(Arc<Puzzle>),
    Err,
}
impl Default for PuzzleCacheEntry {
    fn default() -> Self {
        Self::Building {
            notify: NotifyWhenDropped::new(),
            status: None,
        }
    }
}
impl PuzzleCacheEntry {
    fn build_status_mut(&mut self) -> Option<&mut PuzzleBuildStatus> {
        match self {
            PuzzleCacheEntry::Building { status, .. } => status.as_mut(),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct PuzzleBuildStatus {}

#[derive(Debug, Default)]
pub struct NotifyWhenDropped(Arc<(Mutex<bool>, Condvar)>);
impl NotifyWhenDropped {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn waiter(&self) -> Waiter {
        Waiter(Arc::clone(&self.0))
    }
}
impl Drop for NotifyWhenDropped {
    fn drop(&mut self) {
        let (mutex, condvar) = &*self.0;
        *mutex.lock() = true;
        condvar.notify_all();
    }
}

#[derive(Debug, Clone)]
pub struct Waiter(Arc<(Mutex<bool>, Condvar)>);
impl Waiter {
    pub fn wait(self) {
        let (mutex, condvar) = &*self.0;
        condvar.wait_while(&mut mutex.lock(), |is_done| *is_done);
    }
}
