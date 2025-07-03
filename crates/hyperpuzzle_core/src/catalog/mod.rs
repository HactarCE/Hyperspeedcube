//! Catalog of puzzles and related objects, along with functionality for loading
//! them.

use std::collections::{BTreeSet, HashMap, hash_map};
use std::fmt;
use std::sync::Arc;

use parking_lot::{Mutex, MutexGuard};
use serde::Serialize;

mod db;
mod entry;
mod object;
mod params;
mod specs;
mod subcatalog;

use db::*;
pub use entry::*;
pub use object::*;
pub use params::*;
pub use specs::*;
pub use subcatalog::*;

use crate::{ColorSystem, LogLine, Logger, Puzzle, TagSet, TwistSystem, Version};

/// Catalog of shapes, puzzles, twist systems, etc.
///
/// The database is stored inside an `Arc<Mutex<T>>` so cloning this is cheap.
#[derive(Default, Clone)]
pub struct Catalog {
    /// Database of objects in the catalog.
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

    /// Locks the database.
    ///
    /// **WARNING: This is a low-level operation and can cause deadlocks. Prefer
    /// higher-level methods if possible.**
    pub fn lock_db(&self) -> MutexGuard<'_, Db> {
        self.db.lock()
    }

    /// Returns a list of puzzle specs, including examples for generators, in an
    /// unspecified order.
    ///
    /// Shorthand for `self.object_specs::<Puzzle>()`.
    pub fn puzzle_specs(&self) -> Arc<Vec<Arc<PuzzleSpec>>> {
        self.object_specs::<Puzzle>()
    }

    /// Returns the specification for the object with the given ID, if it
    /// exists. This does not find generated objects.
    pub fn get_spec<T: CatalogObject>(&self, id: &str) -> Option<Arc<T::Spec>> {
        let mut db = self.db.lock();
        db.get_mut::<T>().loaded_specs.get(id).map(Arc::clone)
    }
    /// Returns the generator with the given ID, if it exists.
    pub fn get_generator<T: CatalogObject>(&self, id: &str) -> Option<Arc<T::SpecGenerator>> {
        let mut db = self.db.lock();
        db.get_mut::<T>().loaded_generators.get(id).map(Arc::clone)
    }

    /// Returns a list of object specs, including examples for generators, in an
    /// unspecified order.
    pub fn object_specs<T: CatalogObject>(&self) -> Arc<Vec<Arc<T::Spec>>> {
        self.db.lock().get_mut::<T>().specs()
    }

    /// Returns all entries that might show in the puzzle list, in an
    /// unspecified order: non-generated puzzles, example puzzles, and
    /// puzzle generators.
    pub fn puzzle_list_entries(&self) -> Arc<Vec<Arc<PuzzleListMetadata>>> {
        self.db.lock().puzzles.puzzle_list_entries()
    }

    /// Returns a sorted list of all puzzle authors.
    pub fn authors(&self) -> Vec<String> {
        self.db.lock().authors.iter().cloned().collect()
    }

    /// Adds a puzzle to the catalog.
    pub fn add_puzzle(&self, spec: Arc<PuzzleSpec>) -> eyre::Result<()> {
        let mut db = self.db.lock();
        db.puzzles.add_spec(Arc::clone(&spec))?;
        db.authors.extend(spec.meta.tags.authors().iter().cloned());
        Ok(())
    }
    /// Adds a puzzle generator to the catalog.
    pub fn add_puzzle_generator(&self, spec: Arc<PuzzleSpecGenerator>) -> eyre::Result<()> {
        let mut db = self.db.lock();
        db.puzzles.add_spec_generator(Arc::clone(&spec))?;
        db.authors.extend(spec.meta.tags.authors().iter().cloned());
        Ok(())
    }

    /// Adds a color system to the catalog.
    pub fn add_color_system(&self, colors: Arc<ColorSystem>) -> eyre::Result<()> {
        self.db.lock().color_systems.add_spec(colors)
    }
    /// Adds a color system generator to the catalog.
    pub fn add_color_system_generator(
        &self,
        colors_gen: Arc<ColorSystemGenerator>,
    ) -> eyre::Result<()> {
        self.db.lock().color_systems.add_spec_generator(colors_gen)
    }

    /// Adds a twist system to the catalog.
    pub fn add_twist_system(&self, twists: Arc<TwistSystemSpec>) -> eyre::Result<()> {
        self.db.lock().twist_systems.add_spec(twists)
    }
    /// Adds a twist system generator to the catalog.
    pub fn add_twist_system_generator(
        &self,
        twists_gen: Arc<TwistSystemSpecGenerator>,
    ) -> eyre::Result<()> {
        self.db.lock().twist_systems.add_spec_generator(twists_gen)
    }

    /// Requests an object to be built if it has not been built already, and
    /// then immediately returns the cache entry for the object.
    ///
    /// It may take time for the object to build. If you want to block the
    /// current thread, see [`Self::build_blocking()`].
    ///
    /// # Example
    ///
    /// ```ignore
    /// let rubiks_cube = catalog.build::<Puzzle>("ft_cube:3");
    /// println!("Requested Rubik's cube ...");
    /// loop {
    ///     let cache_entry_guard = rubiks_cube.lock();
    ///     match &*cache_entry_guard {
    ///         CacheEntry::NotStarted => {
    ///             std::thread::sleep(std::time::Duration::from_secs(1));
    ///             continue;
    ///         }
    ///         CacheEntry::Building { notify, .. } => {
    ///             let waiter = notify.waiter();
    ///             drop(cache_entry_guard);
    ///             waiter.wait();
    ///             continue;
    ///         }
    ///         CacheEntry::Ok(_) => {
    ///             println!("Success!");
    ///             break;
    ///         }
    ///         CacheEntry::Err(e) => {
    ///             println!("Error: {e}");
    ///             break;
    ///         }
    ///     }
    /// }
    /// ```
    pub fn build<T: CatalogObject>(&self, id: &str) -> Arc<Mutex<CacheEntry<T>>> {
        let id = id.to_owned();
        let mut db = self.db.lock();
        self.build_non_blocking(
            id.clone(),
            db.get_mut::<T>().objects.entry(id.clone()),
            move |this| this.build_object_generic_blocking::<T>(&id),
        )
    }

    /// Builds an object and blocks the current thread until it is complete.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let rubiks_cube_result = catalog.build_blocking::<Puzzle>("ft_cube:3");
    /// match rubiks_cube_result {
    ///     Ok(_puzzle) => println!("Success!"),
    ///     Err(e) => println!("Error: {e}"),
    /// }
    /// ```
    pub fn build_blocking<T: CatalogObject>(&self, id: &str) -> Result<Arc<T>, String> {
        self.build_object_generic_blocking(id)
    }

    /// Requests an object specification to be generated if it has not been
    /// generated already, and then returns the cache entry for the spec.
    ///
    /// It may take time to generate the spec. If you want to block the current
    /// thread, see [`Self::build_spec_blocking()`].
    pub fn build_spec<T: CatalogObject>(&self, id: &str) -> Arc<Mutex<CacheEntry<T::Spec>>> {
        let id = id.to_owned();
        let mut db = self.db.lock();
        self.build_non_blocking(
            id.clone(),
            db.get_mut::<T>().generated_specs.entry(id.clone()),
            move |this| this.build_spec_blocking::<T>(&id),
        )
    }
    /// Builds an object specification and blocks the current thread until it is
    /// complete.
    pub fn build_spec_blocking<T: CatalogObject>(&self, id: &str) -> Result<Arc<T::Spec>, String> {
        // Start building an object spec on another thread and return
        // immediately
        self.build_spec_generic_blocking::<T>(id)
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
            let mut db = self.db.lock();
            let subcatalog = db.get_mut::<T>();
            Arc::clone(subcatalog.generated_specs.entry(id.to_owned()).or_default())
        };

        let build_fn = |id: &str, progress: &Arc<Mutex<Progress>>| {
            let mut db_guard = self.db.lock();
            let subcatalog = db_guard.get_mut::<T>();
            // Get the object spec, which may be expensive.
            progress.lock().task = BuildTask::GeneratingSpec;
            let generator_output = match crate::parse_generated_id(id) {
                None => match subcatalog.loaded_specs.get(id).cloned() {
                    None => Err(format!("no {} with ID {id:?}", T::NAME)),
                    Some(spec) => {
                        drop(db_guard);
                        Ok(Redirectable::Direct(spec))
                    }
                },
                Some((generator_id, params)) => {
                    match subcatalog.loaded_generators.get(generator_id).cloned() {
                        None => Err(format!("no {} generator with ID {generator_id:?}", T::NAME)),
                        Some(generator) => {
                            drop(db_guard); // unlock mutex before running user code
                            let ctx = BuildCtx::new(&self.default_logger, progress);
                            log::trace!("generating spec for {generator_id:?} {params:?}");
                            let params = params.into_iter().map(|s| s.to_owned()).collect();
                            T::generate_spec(ctx, &generator, params)
                        }
                    }
                }
            };
            match generator_output {
                Ok(ok) => CacheEntry::Ok(ok),
                Err(e) => {
                    self.default_logger.log(LogLine {
                        level: log::Level::Error,
                        msg: format!("error building {id:?}"),
                        full: Some(e.clone()).filter(|s| !s.is_empty()),
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
            let mut db = self.db.lock();
            Arc::clone(db.get_mut::<T>().objects.entry(id.to_owned()).or_default())
        };

        let build_fn = |id: &str, progress: &Arc<Mutex<Progress>>| {
            // Get the object spec, which may be expensive.
            progress.lock().task = BuildTask::GeneratingSpec;
            let spec = match self.build_spec_generic_blocking::<T>(id) {
                Ok(spec) => spec,
                Err(e) => return CacheEntry::Err(e),
            };
            // Redirect if necessary.
            let new_id = spec.id();
            if new_id != id {
                return CacheEntry::Ok(Redirectable::Redirect(new_id.to_owned()));
            }
            // Build the object, which may be expensive.
            let ctx = BuildCtx::new(&self.default_logger, progress);
            let result = T::build_object_from_spec(ctx.clone(), &spec);
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

fn unqualified_type_name<T>() -> &'static str {
    let type_name = std::any::type_name::<T>();
    type_name.rsplit(':').next().unwrap_or(type_name)
}
