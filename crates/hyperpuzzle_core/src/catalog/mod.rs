//! Catalog of puzzles and related objects, along with functionality for loading
//! them.

use std::collections::{BTreeSet, HashMap, hash_map};
use std::sync::Arc;

use parking_lot::{Mutex, MutexGuard};

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
    pub fn add_puzzle(&self, spec: Arc<PuzzleSpec>) -> eyre::Result<()> {
        crate::validate_id(&spec.meta.id)?;
        let mut db = self.db.lock();
        db.authors.extend(spec.meta.tags.authors().iter().cloned());
        db.puzzles.insert(spec.meta.id.clone(), spec);
        Ok(())
    }
    /// Adds a puzzle generator to the catalog.
    pub fn add_puzzle_generator(&self, spec: Arc<PuzzleSpecGenerator>) -> eyre::Result<()> {
        crate::validate_id(&spec.meta.id)?;
        let mut db = self.db.lock();
        db.authors.extend(spec.meta.tags.authors().iter().cloned());
        db.puzzle_generators.insert(spec.meta.id.clone(), spec);
        Ok(())
    }
    /// Adds a color system to the catalog.
    pub fn add_color_system(&self, colors: Arc<ColorSystem>) -> eyre::Result<()> {
        crate::validate_id(&colors.id)?;
        self.db
            .lock()
            .color_systems
            .insert(colors.id.clone(), Arc::clone(&colors));
        // Automatically cache it
        self.db.lock().color_system_cache.insert(
            colors.id.clone(),
            Arc::new(Mutex::new(CacheEntry::Ok(Redirectable::Direct(colors)))),
        );
        Ok(())
    }
    /// Adds a color system generator to the catalog.
    pub fn add_color_system_generator(
        &self,
        colors_generator: Arc<ColorSystemGenerator>,
    ) -> eyre::Result<()> {
        crate::validate_id(&colors_generator.id)?;
        self.db
            .lock()
            .color_system_generators
            .insert(colors_generator.id.clone(), colors_generator);
        Ok(())
    }

    /// Requests a puzzle to be built if it has not been built already, and then
    /// returns the cache entry for the puzzle.
    ///
    /// It may take time for the puzzle to build. If you want to block the
    /// current thread, see [`Self::build_puzzle_blocking()`].
    pub fn build_puzzle(&self, id: &str) -> Arc<Mutex<CacheEntry<Puzzle>>> {
        self.build_object_generic(id)
    }
    /// Builds a puzzle and blocks the current thread until it is complete.
    pub fn build_puzzle_blocking(&self, id: &str) -> Result<Arc<Puzzle>, String> {
        self.build_object_generic_blocking(id)
    }

    /// Requests a puzzle to be built if it has not been built already, and then
    /// returns the cache entry for the puzzle.
    ///
    /// It may take time for the puzzle to build. If you want to block the
    /// current thread, see [`Self::build_puzzle_spec_blocking()`].
    pub fn build_puzzle_spec(&self, id: &str) -> Arc<Mutex<CacheEntry<PuzzleSpec>>> {
        self.build_spec_generic::<Puzzle>(id)
    }
    /// Builds a puzzle spec and blocks the current thread until it is complete.
    pub fn build_puzzle_spec_blocking(&self, id: &str) -> Result<Arc<PuzzleSpec>, String> {
        self.build_spec_generic_blocking::<Puzzle>(id)
    }

    /// Requests a color system to be built if it has not been built already,
    /// and then returns the cache entry for the color system.
    ///
    /// It may take time for the color system to build. If you want to block the
    /// current thread, see [`Self::build_color_system_blocking()`].
    pub fn build_color_system(&self, id: &str) -> Arc<Mutex<CacheEntry<ColorSystem>>> {
        self.build_object_generic::<ColorSystem>(id)
    }
    /// Builds a puzzle and blocks the current thread until it is complete.
    pub fn build_color_system_blocking(&self, id: &str) -> Result<Arc<ColorSystem>, String> {
        self.build_object_generic_blocking::<ColorSystem>(id)
    }

    /// Starts building an object spec on another thread and returns
    /// immediately.
    fn build_spec_generic<T: CatalogObject>(&self, id: &str) -> Arc<Mutex<CacheEntry<T::Spec>>> {
        let id = id.to_owned();
        self.build_non_blocking(
            id.clone(),
            T::get_spec_cache(&mut self.db.lock()).entry(id.clone()),
            move |this| this.build_spec_generic_blocking::<T>(&id),
        )
    }

    /// Starts building an object on another thread and returns immediately.
    fn build_object_generic<T: CatalogObject>(&self, id: &str) -> Arc<Mutex<CacheEntry<T>>> {
        let id = id.to_owned();
        self.build_non_blocking(
            id.clone(),
            T::get_cache(&mut self.db.lock()).entry(id.clone()),
            move |this| this.build_object_generic_blocking::<T>(&id),
        )
    }

    fn build_non_blocking<T>(
        &self,
        id: String,
        entry: hash_map::Entry<'_, String, Arc<Mutex<CacheEntry<T>>>>,
        build_fn: impl 'static + Send + FnOnce(Self) -> Result<Arc<T>, String>,
    ) -> Arc<Mutex<CacheEntry<T>>> {
        match entry {
            hash_map::Entry::Occupied(e) => Arc::clone(e.get()),
            hash_map::Entry::Vacant(e) => {
                let this = self.clone();
                std::thread::spawn(move || {
                    if let Err(e) = build_fn(this) {
                        log::error!("error building {id:?}: {e}");
                    }
                });
                Arc::clone(e.insert(Arc::new(Mutex::new(CacheEntry::NotStarted))))
            }
        }
    }

    fn build_spec_generic_blocking<T: CatalogObject>(
        &self,
        initial_id: &str,
    ) -> Result<Arc<T::Spec>, String> {
        let get_cache_entry_fn = |id: &str| {
            Arc::clone(
                T::get_spec_cache(&mut self.db.lock())
                    .entry(id.to_owned())
                    .or_default(),
            )
        };

        let build_fn = |id: &str, progress: &Arc<Mutex<Progress>>| {
            let mut db_guard = self.db.lock();
            let mut file = None;
            // Get the object spec, which may be expensive.
            progress.lock().task = BuildTask::GeneratingSpec;
            let generator_output = match crate::parse_generated_id(id) {
                None => match T::get_specs(&mut db_guard).get(id).cloned() {
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
                            let mut ctx = BuildCtx::new(&self.default_logger, progress);
                            log::trace!("generating spec for {generator_id:?} {params:?}");
                            T::generate_spec(&mut ctx, &generator, params)
                        }
                    }
                }
            };
            match generator_output {
                Ok(ok) => CacheEntry::Ok(ok),
                Err(e) => {
                    let msg = format!("error building {id:?}: {e}");
                    self.default_logger.log(LogLine {
                        level: log::Level::Error,
                        // file,
                        msg,
                        // traceback: None,
                    });
                    CacheEntry::Err(e)
                }
            }
        };

        self.build_generic_with_redirect_handling(
            initial_id.to_owned(),
            &mut vec![],
            &get_cache_entry_fn,
            &build_fn,
        )
    }

    fn build_object_generic_blocking<T: CatalogObject>(&self, id: &str) -> Result<Arc<T>, String> {
        let get_cache_entry_fn = |id: &str| {
            Arc::clone(
                T::get_cache(&mut self.db.lock())
                    .entry(id.to_owned())
                    .or_default(),
            )
        };

        let build_fn = |id: &str, progress: &Arc<Mutex<Progress>>| {
            // Get the object spec, which may be expensive.
            progress.lock().task = BuildTask::GeneratingSpec;
            let spec = match self.build_spec_generic_blocking::<T>(id) {
                Ok(spec) => spec,
                Err(e) => return CacheEntry::Err(e),
            };
            // Redirect if necessary.
            let new_id = T::get_spec_id(&spec);
            if new_id != id {
                return CacheEntry::Ok(Redirectable::Redirect(new_id.to_owned()));
            }
            // Build the object, which may be expensive.
            let mut ctx = BuildCtx::new(&self.default_logger, progress);
            let result = T::build_object_from_spec(&mut ctx, &spec);
            if let Err(e) = &result {
                ctx.logger.error(e);
            }
            CacheEntry::from(result)
        };

        self.build_generic_with_redirect_handling(
            id.to_owned(),
            &mut vec![],
            &get_cache_entry_fn,
            &build_fn,
        )
    }

    fn build_generic_with_redirect_handling<T>(
        &self,
        id: String,
        redirect_sequence: &mut Vec<String>,
        get_cache_entry_fn: &impl Fn(&str) -> Arc<Mutex<CacheEntry<T>>>,
        build_fn: &impl Fn(&str, &Arc<Mutex<Progress>>) -> CacheEntry<T>,
    ) -> Result<Arc<T>, String> {
        let type_str = unqualified_type_name::<T>();

        log::trace!("requesting {type_str} {id:?}");
        if !redirect_sequence.is_empty() {
            log::trace!("(redirected from {redirect_sequence:?})");
        }

        redirect_sequence.push(id.clone());
        if redirect_sequence.len() > crate::MAX_ID_REDIRECTS {
            let msg = format!("too many ID redirects: {redirect_sequence:?}");
            self.default_logger.error(&msg);
            return Err(msg);
        }

        let cache_entry = get_cache_entry_fn(&id);
        let mut cache_entry_guard = cache_entry.lock();

        if let CacheEntry::NotStarted = &*cache_entry_guard {
            log::trace!("{type_str} {id:?} not yet started");
            // Mark that this object is being built.
            let progress = Arc::new(Mutex::new(Progress::default()));
            *cache_entry_guard = CacheEntry::Building {
                progress: Arc::clone(&progress),
                notify: NotifyWhenDropped::new(),
            };
            log::trace!("building {type_str} {id:?}");
            // Unlock the mutex before during expensive object generation.
            let cache_entry_value =
                MutexGuard::unlocked(&mut cache_entry_guard, || build_fn(&id, &progress));
            log::trace!("storing {type_str} {id:?}");
            // Store the result.
            *cache_entry_guard = cache_entry_value;
        };

        // If another thread is building the object, then wait for that.
        if let CacheEntry::Building { notify, .. } = &mut *cache_entry_guard {
            log::trace!("waiting for another thread to build {type_str} {id:?}");
            let waiter = notify.waiter();
            MutexGuard::unlocked(&mut cache_entry_guard, || {
                waiter.wait();
            });
            log::trace!("done waiting on {id:?}");
        }

        match &*cache_entry_guard {
            // The object was requested but has not started being built.
            CacheEntry::NotStarted => {
                Err("internal error: object did not start building".to_owned())
            }

            // The object has already been built.
            CacheEntry::Ok(Redirectable::Redirect(new_id)) => {
                let new_id = new_id.clone();
                drop(cache_entry_guard);
                self.build_generic_with_redirect_handling::<T>(
                    new_id,
                    redirect_sequence,
                    get_cache_entry_fn,
                    build_fn,
                )
            }
            CacheEntry::Ok(Redirectable::Direct(output)) => Ok(Arc::clone(output)),
            CacheEntry::Err(e) => Err(e.clone()),

            // The object has already been built or is being built.
            CacheEntry::Building { .. } => Err("unexpected Building entry".to_owned()),
        }
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

fn unqualified_type_name<T>() -> &'static str {
    let type_name = std::any::type_name::<T>();
    type_name.rsplit(':').next().unwrap_or(type_name)
}
