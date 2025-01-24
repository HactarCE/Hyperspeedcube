use std::collections::{hash_map, BTreeMap, BTreeSet, HashMap};
use std::sync::Arc;

use itertools::Itertools;
use parking_lot::Mutex;

mod entry;
mod generators;
mod specs;

pub use entry::*;
pub use generators::*;
pub use specs::*;

use crate::{ColorSystem, LogLine, Logger, Puzzle};

#[derive(Default)]
struct Db {
    /// Loaded puzzles by ID.
    puzzles: BTreeMap<String, Arc<PuzzleSpec>>,
    /// Loaded puzzle generators by ID.
    puzzle_generators: BTreeMap<String, Arc<PuzzleSpecGenerator>>,
    /// Cache of constructed puzzles.
    puzzle_cache: HashMap<String, Arc<Mutex<CacheEntry<Puzzle>>>>,

    /// Loaded color systems by ID.
    color_systems: BTreeMap<String, Arc<ColorSystem>>,
    /// Loaded color system generators by ID.
    color_system_generators: BTreeMap<String, Arc<ColorSystemGenerator>>,
    /// Cache of generated color systems.
    color_system_cache: HashMap<String, Arc<Mutex<CacheEntry<ColorSystem>>>>,

    /// Sorted list of all puzzle definition authors.
    authors: BTreeSet<String>,
}

/// Catalog of shapes, puzzles, twist systems, etc.
///
/// The database is stored inside an `Arc<Mutex<T>>` so cloning this is cheap.
#[derive(Default, Clone)]
pub struct Catalog {
    db: Arc<Mutex<Db>>,
    logger: Logger,
}
impl Catalog {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn logger(&self) -> &Logger {
        &self.logger
    }

    pub fn puzzles_and_generator_examples(&self) -> Vec<Arc<PuzzleSpec>> {
        let db = self.db.lock();
        itertools::chain(
            db.puzzles.values(),
            db.puzzle_generators
                .values()
                .flat_map(|g| g.examples.values()),
        )
        .map(Arc::clone)
        .collect()
    }
    pub fn puzzle_generators(&self) -> Vec<Arc<PuzzleSpecGenerator>> {
        self.db
            .lock()
            .puzzle_generators
            .values()
            .map(Arc::clone)
            .collect()
    }
    pub fn color_systems(&self) -> Vec<Arc<ColorSystem>> {
        self.db
            .lock()
            .color_systems
            .values()
            .map(Arc::clone)
            .collect_vec()
    }
    pub fn authors(&self) -> Vec<String> {
        self.db.lock().authors.iter().cloned().collect()
    }

    pub fn get_puzzle_generator(&self, id: &str) -> Option<Arc<PuzzleSpecGenerator>> {
        self.db.lock().puzzle_generators.get(id).map(Arc::clone)
    }

    pub fn add_puzzle(&self, spec: Arc<PuzzleSpec>) {
        let mut db = self.db.lock();
        db.authors.extend(spec.meta.tags.authors().iter().cloned());
        db.puzzles.insert(spec.meta.id.clone(), spec);
    }
    pub fn add_puzzle_generator(&self, spec: Arc<PuzzleSpecGenerator>) {
        let mut db = self.db.lock();
        db.authors.extend(spec.meta.tags.authors().iter().cloned());
        db.puzzle_generators.insert(spec.meta.id.clone(), spec);
    }

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
    pub fn add_color_system_generator(&self, colors_generator: Arc<ColorSystemGenerator>) {
        self.db
            .lock()
            .color_system_generators
            .insert(colors_generator.id.clone(), colors_generator);
    }

    pub fn build_puzzle(&self, id: &str) -> Arc<Mutex<CacheEntry<Puzzle>>> {
        self.build_generic(id)
    }
    pub fn build_puzzle_blocking(&self, id: &str) -> Result<Arc<Puzzle>, String> {
        self.build_generic_blocking(id)
    }

    pub fn build_color_system(&self, id: &str) -> Arc<Mutex<CacheEntry<ColorSystem>>> {
        self.build_generic(id)
    }
    pub fn build_color_system_blocking(&self, id: &str) -> Result<Arc<ColorSystem>, String> {
        self.build_generic_blocking(id)
    }

    pub fn clear_cache(&self) {
        self.db.lock().puzzle_cache.clear();
        self.db.lock().color_system_cache.clear();
    }

    fn build_generic<T: CatalogObject>(&self, id: &str) -> Arc<Mutex<CacheEntry<T>>> {
        let id = id.to_owned();
        let mut db_guard = self.db.lock();
        match T::get_cache(&mut db_guard).entry(id.clone()) {
            hash_map::Entry::Occupied(e) => Arc::clone(e.get()),
            hash_map::Entry::Vacant(e) => {
                let this = self.clone();
                std::thread::spawn(move || {
                    if let Err(e) = this.build_generic_blocking::<T>(&id) {
                        log::error!("error building {id:?}: {e}")
                    }
                });
                Arc::clone(e.insert(Arc::new(Mutex::new(CacheEntry::NotStarted))))
            }
        }
    }
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
                                    let ctx = BuildCtx::new(&self.logger, &progress);
                                    T::generate_spec(ctx, &generator, params)
                                }
                            }
                        }
                    };
                    // Build the object, which may be expensive.
                    let cache_entry_value = match generator_output {
                        Ok(Redirectable::Direct(object_spec)) => {
                            let ctx = BuildCtx::new(&self.logger, &progress);
                            CacheEntry::from(T::build_object_from_spec(ctx, &object_spec))
                        }
                        Ok(Redirectable::Redirect(new_id)) => {
                            CacheEntry::Ok(Redirectable::Redirect(new_id))
                        }
                        Err(e) => {
                            let msg = format!("error building {id}: {e}");
                            self.logger.log(LogLine {
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
        self.logger.error(&msg);

        Err(msg)
    }
}

/// Object with an ID (such as a puzzle or color system) that can be stored in
/// the catalog.
trait CatalogObject: Sized {
    type Spec;
    type SpecGenerator;

    const NAME: &str;

    fn get_cache(db: &mut Db) -> &mut HashMap<String, Arc<Mutex<CacheEntry<Self>>>>;
    fn get_specs(db: &mut Db) -> &mut BTreeMap<String, Arc<Self::Spec>>;
    fn get_generators(db: &mut Db) -> &mut BTreeMap<String, Arc<Self::SpecGenerator>>;

    fn get_spec_filename(spec: &Self::Spec) -> Option<String>;
    fn get_generator_filename(generator: &Self::SpecGenerator) -> Option<String>;

    fn build_object_from_spec(
        ctx: BuildCtx,
        spec: &Arc<Self::Spec>,
    ) -> Result<Redirectable<Arc<Self>>, String>;
    fn generate_spec(
        ctx: BuildCtx,
        gen: &Arc<Self::SpecGenerator>,
        params: Vec<&str>,
    ) -> Result<Redirectable<Arc<Self::Spec>>, String>;
}

impl CatalogObject for Puzzle {
    type Spec = PuzzleSpec;
    type SpecGenerator = PuzzleSpecGenerator;

    const NAME: &str = "puzzle";

    fn get_cache(db: &mut Db) -> &mut HashMap<String, Arc<Mutex<CacheEntry<Self>>>> {
        &mut db.puzzle_cache
    }
    fn get_specs(db: &mut Db) -> &mut BTreeMap<String, Arc<Self::Spec>> {
        &mut db.puzzles
    }
    fn get_generators(db: &mut Db) -> &mut BTreeMap<String, Arc<Self::SpecGenerator>> {
        &mut db.puzzle_generators
    }

    fn get_spec_filename(spec: &Self::Spec) -> Option<String> {
        spec.meta.tags.filename().map(str::to_owned)
    }
    fn get_generator_filename(generator: &Self::SpecGenerator) -> Option<String> {
        generator.meta.tags.filename().map(str::to_owned)
    }

    fn build_object_from_spec(
        ctx: BuildCtx,
        spec: &Arc<Self::Spec>,
    ) -> Result<Redirectable<Arc<Self>>, String> {
        (spec.build)(ctx).map_err(|e| format!("{e:#}"))
    }
    fn generate_spec(
        ctx: BuildCtx,
        gen: &Arc<Self::SpecGenerator>,
        params: Vec<&str>,
    ) -> Result<Redirectable<Arc<Self::Spec>>, String> {
        (gen.generate)(ctx, params).map_err(|e| format!("{e:#}"))
    }
}

impl CatalogObject for ColorSystem {
    type Spec = ColorSystem;
    type SpecGenerator = ColorSystemGenerator;

    const NAME: &str = "color system";

    fn get_cache(db: &mut Db) -> &mut HashMap<String, Arc<Mutex<CacheEntry<Self>>>> {
        &mut db.color_system_cache
    }
    fn get_specs(db: &mut Db) -> &mut BTreeMap<String, Arc<Self::Spec>> {
        &mut db.color_systems
    }
    fn get_generators(db: &mut Db) -> &mut BTreeMap<String, Arc<Self::SpecGenerator>> {
        &mut db.color_system_generators
    }

    fn get_spec_filename(_spec: &Self::Spec) -> Option<String> {
        None
    }
    fn get_generator_filename(_generator: &Self::SpecGenerator) -> Option<String> {
        None
    }

    fn build_object_from_spec(
        _ctx: BuildCtx,
        spec: &Arc<Self::Spec>,
    ) -> Result<Redirectable<Arc<Self>>, String> {
        Ok(Redirectable::Direct(Arc::clone(spec)))
    }
    fn generate_spec(
        ctx: BuildCtx,
        gen: &Arc<Self::SpecGenerator>,
        params: Vec<&str>,
    ) -> Result<Redirectable<Arc<Self::Spec>>, String> {
        (gen.generate)(ctx, params).map_err(|e| format!("{e:#}"))
    }
}

/// Possible ID redirect.
#[derive(Debug, Clone)]
pub enum Redirectable<T> {
    /// Thing directly generated.
    Direct(T),
    /// Redirect to a different ID.
    Redirect(String),
}

pub enum IdError {
    NoPuzzle,
    NoGenerator,
    BadParams, // TODO: more detail
    TooManyRedirects(Vec<String>),
}
