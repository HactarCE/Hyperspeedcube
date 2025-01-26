use std::collections::{hash_map, BTreeSet, HashMap};
use std::sync::Arc;

use parking_lot::Mutex;

mod db;
mod entry;
mod params;
mod specs;

use db::*;
pub use entry::*;
pub use params::*;
pub use specs::*;

use crate::{ColorSystem, LogLine, Logger, Puzzle};

/// Catalog of shapes, puzzles, twist systems, etc.
///
/// The database is stored inside an `Arc<Mutex<T>>` so cloning this is cheap.
#[derive(Default, Clone)]
pub struct Catalog {
    db: Arc<Mutex<Db>>,
    // TODO: consider removing the logger here
    default_logger: Logger,
}
impl Catalog {
    /// Constructs a new empty catalog.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the default logger for the catalog.
    pub fn default_logger(&self) -> &Logger {
        &self.default_logger
    }

    /// Returns a snapshot of the current puzzle catalog.
    ///
    /// This is slightly expensive, so don't call it in a hot loop.
    pub fn puzzles(&self) -> PuzzleCatalog {
        self.subcatalog()
    }
    /// Returns a snapshot of the current color system catalog.
    ///
    /// This is slightly expensive, so don't call it in a hot loop.
    pub fn color_systems(&self) -> ColorSystemCatalog {
        self.subcatalog()
    }
    fn subcatalog<T: CatalogObject>(&self) -> SubCatalog<T> {
        let mut db = self.db.lock();
        SubCatalog {
            generators: T::get_generators(&mut db).clone(),
            generated_examples: T::get_generators(&mut db)
                .values()
                .filter_map(|g| T::get_generator_examples(g))
                .flatten()
                .map(|(k, v)| (k.clone(), Arc::clone(v)))
                .collect(),
            non_generated: T::get_specs(&mut db).clone(),
        }
    }

    /// Returns a sorted list of all puzzle authors.
    pub fn authors(&self) -> Vec<String> {
        self.db.lock().authors.iter().cloned().collect()
    }

    /// Adds a puzzle to the catalog.
    pub fn add_puzzle(&self, spec: Arc<PuzzleSpec>) {
        let mut db = self.db.lock();
        db.authors.extend(spec.meta.tags.authors().iter().cloned());
        db.puzzles.insert(spec.meta.id.clone(), spec);
    }
    /// Adds a puzzle generator to the catalog.
    pub fn add_puzzle_generator(&self, spec: Arc<PuzzleSpecGenerator>) {
        let mut db = self.db.lock();
        db.authors.extend(spec.meta.tags.authors().iter().cloned());
        db.puzzle_generators.insert(spec.meta.id.clone(), spec);
    }
    /// Adds a color system to the catalog.
    pub fn add_color_system(&self, colors: Arc<ColorSystem>) {
        self.db
            .lock()
            .color_systems
            .insert(colors.id.clone(), Arc::clone(&colors));
        // Automatically cache it
        self.db.lock().color_system_cache.insert(
            colors.id.clone(),
            Arc::new(Mutex::new(CacheEntry::Ok(Redirectable::Direct(colors)))),
        );
    }
    /// Adds a color system generator to the catalog.
    pub fn add_color_system_generator(&self, colors_generator: Arc<ColorSystemGenerator>) {
        self.db
            .lock()
            .color_system_generators
            .insert(colors_generator.id.clone(), colors_generator);
    }

    /// Requests a puzzle to be built if it has not been built already, and then
    /// returns the cache entry for the puzzle.
    ///
    /// It may take time for the puzzle to build. If you want to block the
    /// current thread, see [`Self::build_puzzle_blocking()`].
    pub fn build_puzzle(&self, id: &str) -> Arc<Mutex<CacheEntry<Puzzle>>> {
        self.build_generic(id)
    }
    /// Builds a puzzle and blocks the current thread until it is complete.
    pub fn build_puzzle_blocking(&self, id: &str) -> Result<Arc<Puzzle>, String> {
        self.build_generic_blocking(id)
    }

    /// Requests a color system to be built if it has not been built already,
    /// and then returns the cache entry for the color system.
    ///
    /// It may take time for the color system to build. If you want to block the
    /// current thread, see [`Self::build_color_system_blocking()`].
    pub fn build_color_system(&self, id: &str) -> Arc<Mutex<CacheEntry<ColorSystem>>> {
        self.build_generic(id)
    }
    /// Builds a puzzle and blocks the current thread until it is complete.
    pub fn build_color_system_blocking(&self, id: &str) -> Result<Arc<ColorSystem>, String> {
        self.build_generic_blocking(id)
    }

    /// Builds an object and blocks the current thread until it is complete.
    fn build_generic<T: CatalogObject>(&self, id: &str) -> Arc<Mutex<CacheEntry<T>>> {
        let id = id.to_owned();
        let mut db_guard = self.db.lock();
        match T::get_cache(&mut db_guard).entry(id.clone()) {
            hash_map::Entry::Occupied(e) => Arc::clone(e.get()),
            hash_map::Entry::Vacant(e) => {
                let this = self.clone();
                std::thread::spawn(move || {
                    if let Err(e) = this.build_generic_blocking::<T>(&id) {
                        log::error!("error building {id:?}: {e}");
                    }
                });
                Arc::clone(e.insert(Arc::new(Mutex::new(CacheEntry::NotStarted))))
            }
        }
    }
    /// Builds an object on the current thread, or returns early if the object
    /// is already being constructed on another thread. This may or may not
    /// block the current thread.
    fn build_generic_blocking<T: CatalogObject>(&self, id: &str) -> Result<Arc<T>, String> {
        let mut id = id.to_owned();
        let mut redirect_sequence = vec![];

        for _ in 0..crate::MAX_ID_REDIRECTS {
            redirect_sequence.push(id.clone());

            let mut db_guard = self.db.lock();

            let mut file = None;

            let cache_entry =
                Arc::clone(T::get_cache(&mut db_guard).entry(id.clone()).or_default());
            let mut cache_entry_guard = cache_entry.lock();

            // If another thread is building the object, then wait for that.
            while let CacheEntry::Building { notify, .. } = &*cache_entry_guard {
                log::trace!("waiting for another thread to build {id:?}");
                let waiter = notify.waiter();
                drop(cache_entry_guard);
                waiter.wait();
                cache_entry_guard = cache_entry.lock();
                log::trace!("done waiting on {id:?}");
            }

            match &mut *cache_entry_guard {
                // The object was requested but has not started being built.
                CacheEntry::NotStarted => {
                    // Mark that this object is being built.
                    let progress = Arc::new(Mutex::new(Progress::default()));
                    *cache_entry_guard = CacheEntry::Building {
                        progress: Arc::clone(&progress),
                        notify: NotifyWhenDropped::new(),
                    };
                    // Unlock the mutex before any expensive object generation.
                    drop(cache_entry_guard);
                    // Get the object spec, which may be expensive.
                    progress.lock().task = BuildTask::GeneratingSpec;
                    let generator_output = match crate::parse_generated_id(&id) {
                        None => match T::get_specs(&mut db_guard).get(&id).cloned() {
                            None => Err(format!("no {} with ID {id:?}", T::NAME)),
                            Some(spec) => {
                                drop(db_guard);
                                file = T::get_spec_filename(&spec);
                                Ok(Redirectable::Direct(spec))
                            }
                        },
                        Some((generator_id, params)) => {
                            match T::get_generators(&mut db_guard).get(generator_id).cloned() {
                                None => Err(format!("no generator with ID {generator_id:?}")),
                                Some(generator) => {
                                    drop(db_guard); // unlock mutex before running Lua code
                                    file = T::get_generator_filename(&generator);
                                    let ctx = BuildCtx::new(&self.default_logger, &progress);
                                    T::generate_spec(ctx, &generator, params)
                                }
                            }
                        }
                    };
                    // Build the object, which may be expensive.
                    let cache_entry_value = match generator_output {
                        Ok(Redirectable::Direct(object_spec)) => {
                            let ctx = BuildCtx::new(&self.default_logger, &progress);
                            CacheEntry::from(T::build_object_from_spec(ctx, &object_spec))
                        }
                        Ok(Redirectable::Redirect(new_id)) => {
                            CacheEntry::Ok(Redirectable::Redirect(new_id))
                        }
                        Err(e) => {
                            let msg = format!("error building {id}: {e}");
                            self.default_logger.log(LogLine {
                                level: log::Level::Error,
                                file,
                                msg,
                                traceback: None,
                            });
                            CacheEntry::Err(e)
                        }
                    };
                    let mut cache_entry_guard = cache_entry.lock();
                    *cache_entry_guard = cache_entry_value;
                    match &*cache_entry_guard {
                        CacheEntry::NotStarted => {
                            return Err("internal error: object did not start building".to_owned());
                        }
                        CacheEntry::Building { notify, .. } => {
                            let waiter = notify.waiter();
                            drop(cache_entry_guard);
                            waiter.wait();
                        }
                        CacheEntry::Ok(Redirectable::Direct(output)) => {
                            return Ok(Arc::clone(output));
                        }
                        CacheEntry::Ok(Redirectable::Redirect(new_id)) => {
                            id = new_id.clone();
                        }
                        CacheEntry::Err(e) => return Err(e.clone()),
                    }
                }

                // The object has already been built.
                CacheEntry::Ok(Redirectable::Redirect(new_id)) => id = new_id.clone(),
                CacheEntry::Ok(Redirectable::Direct(output)) => return Ok(Arc::clone(output)),
                CacheEntry::Err(e) => return Err(e.clone()),

                // The object has already been built or is being built.
                CacheEntry::Building { .. } => return Err("unexpected Building entry".to_owned()),
            }
        }

        let msg = format!("too many ID redirects: {redirect_sequence:?}");
        self.default_logger.error(&msg);

        Err(msg)
    }
}

/// List of all puzzles and puzzle generators in a catalog.
pub type PuzzleCatalog = SubCatalog<Puzzle>;

/// List of all color systems and color system generators in a catalog.
pub type ColorSystemCatalog = SubCatalog<ColorSystem>;

/// List of all objects and generators for a specific kind of catalog object.
pub struct SubCatalog<T: CatalogObject> {
    /// List of generators.
    pub generators: HashMap<String, Arc<T::SpecGenerator>>,
    /// List of examples for generators.
    pub generated_examples: HashMap<String, Arc<T::Spec>>,
    /// List of non-generated objects, in an unspecified order.
    pub non_generated: HashMap<String, Arc<T::Spec>>,
}
impl<T: CatalogObject> SubCatalog<T> {
    /// Returns a list of objects, including generator examples, in an
    /// unspecified order.
    pub fn objects(&self) -> impl Iterator<Item = &Arc<T::Spec>> {
        itertools::chain(
            self.non_generated.values(),
            self.generated_examples.values(),
        )
    }
}
impl PuzzleCatalog {
    /// Returns all entries that might show in the puzzle list, in an
    /// unspecified order: non-generated puzzles, example puzzles, and
    /// puzzle generators.
    pub fn puzzle_list_entries(&self) -> impl Iterator<Item = &PuzzleListMetadata> {
        itertools::chain(
            self.generators.values().map(|g| &g.meta),
            self.objects().map(|o| &o.meta),
        )
    }
}
