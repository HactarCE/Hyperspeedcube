use super::*;

#[derive(Default)]
#[doc(hidden)]
pub struct Db {
    /// Loaded puzzle generators by ID.
    pub(super) puzzle_generators: HashMap<String, Arc<PuzzleSpecGenerator>>,
    /// Loaded puzzles by ID.
    pub(super) puzzles: HashMap<String, Arc<PuzzleSpec>>,
    /// Cache of constructed puzzles.
    pub(super) puzzle_cache: HashMap<String, Arc<Mutex<CacheEntry<Puzzle>>>>,

    /// Loaded color system generators by ID.
    pub(super) color_system_generators: HashMap<String, Arc<ColorSystemGenerator>>,
    /// Loaded color systems by ID.
    pub(super) color_systems: HashMap<String, Arc<ColorSystem>>,
    /// Cache of generated color systems.
    pub(super) color_system_cache: HashMap<String, Arc<Mutex<CacheEntry<ColorSystem>>>>,

    /// Sorted list of all puzzle definition authors.
    pub(super) authors: BTreeSet<String>,
}

/// Object with an ID (such as a puzzle or color system) that can be stored
/// in the catalog.
#[doc(hidden)]
pub trait CatalogObject: Sized {
    type Spec;
    type SpecGenerator;

    const NAME: &str;

    fn get_cache(db: &mut Db) -> &mut HashMap<String, Arc<Mutex<CacheEntry<Self>>>>;
    fn get_specs(db: &mut Db) -> &mut HashMap<String, Arc<Self::Spec>>;
    fn get_generators(db: &mut Db) -> &mut HashMap<String, Arc<Self::SpecGenerator>>;

    fn get_spec_filename(spec: &Self::Spec) -> Option<String>;
    fn get_generator_filename(generator: &Self::SpecGenerator) -> Option<String>;
    fn get_generator_examples(
        generator: &Self::SpecGenerator,
    ) -> Option<&HashMap<String, Arc<Self::Spec>>>;

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
    fn get_specs(db: &mut Db) -> &mut HashMap<String, Arc<Self::Spec>> {
        &mut db.puzzles
    }
    fn get_generators(db: &mut Db) -> &mut HashMap<String, Arc<Self::SpecGenerator>> {
        &mut db.puzzle_generators
    }

    fn get_spec_filename(spec: &Self::Spec) -> Option<String> {
        spec.meta.tags.filename().map(str::to_owned)
    }
    fn get_generator_filename(generator: &Self::SpecGenerator) -> Option<String> {
        generator.meta.tags.filename().map(str::to_owned)
    }
    fn get_generator_examples(
        generator: &Self::SpecGenerator,
    ) -> Option<&HashMap<String, Arc<Self::Spec>>> {
        Some(&generator.examples)
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
    fn get_specs(db: &mut Db) -> &mut HashMap<String, Arc<Self::Spec>> {
        &mut db.color_systems
    }
    fn get_generators(db: &mut Db) -> &mut HashMap<String, Arc<Self::SpecGenerator>> {
        &mut db.color_system_generators
    }

    fn get_spec_filename(_spec: &Self::Spec) -> Option<String> {
        None
    }
    fn get_generator_filename(_generator: &Self::SpecGenerator) -> Option<String> {
        None
    }
    fn get_generator_examples(
        _generator: &Self::SpecGenerator,
    ) -> Option<&HashMap<String, Arc<Self::Spec>>> {
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
