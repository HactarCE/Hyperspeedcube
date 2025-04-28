use super::*;

/// Object with an ID.
pub trait HasId: Sized {
    /// Returns the ID.
    fn id(&self) -> &str;
}
macro_rules! impl_has_id {
    ($type:ty, $($tok:tt)*) => {
        impl HasId for $type {
            fn id(&self) -> &str {
                &self.$($tok)*
            }
        }
    };
}
impl_has_id!(Puzzle, meta.id);
impl_has_id!(PuzzleSpec, meta.id);
impl_has_id!(PuzzleSpecGenerator, meta.id);
impl_has_id!(ColorSystem, id);
impl_has_id!(ColorSystemGenerator, id);
impl_has_id!(TwistSystem, id);
impl_has_id!(TwistSystemSpecGenerator, id);

/// Object with an ID (such as a puzzle or color system) that can be stored in
/// the catalog.
pub trait CatalogObject: HasId + CatalogObjectImpl {
    /// Returns the subcatalog containing this object type.
    fn get_subcatalog(db: &Db) -> &SubCatalog<Self>;
    /// Returns a mutable reference to the subcatalog containing this object
    /// type.
    fn get_subcatalog_mut(db: &mut Db) -> &mut SubCatalog<Self>;
}

#[doc(hidden)]
pub trait CatalogObjectImpl: Sized + HasId {
    type Spec: HasId;
    type SpecGenerator: HasId;

    const NAME: &str;

    fn get_spec_filename(spec: &Self::Spec) -> Option<String>;
    fn get_spec_authors(_spec: &Self::Spec) -> impl IntoIterator<Item = &String> {
        []
    }
    fn get_generator_filename(generator: &Self::SpecGenerator) -> Option<String>;
    fn get_generator_examples(
        generator: &Self::SpecGenerator,
    ) -> Option<&HashMap<String, Arc<Self::Spec>>>;
    fn get_generator_authors(
        _generator: &Self::SpecGenerator,
    ) -> impl IntoIterator<Item = &String> {
        []
    }

    fn build_object_from_spec(ctx: BuildCtx, spec: &Arc<Self::Spec>) -> BuildResult<Self, String>;
    fn generate_spec(
        ctx: BuildCtx,
        generator: &Arc<Self::SpecGenerator>,
        params: Vec<String>,
    ) -> BuildResult<Self::Spec, String>;
}

impl CatalogObject for Puzzle {
    fn get_subcatalog(db: &Db) -> &SubCatalog<Self> {
        &db.puzzles
    }
    fn get_subcatalog_mut(db: &mut Db) -> &mut SubCatalog<Self> {
        &mut db.puzzles
    }
}
impl CatalogObjectImpl for Puzzle {
    type Spec = PuzzleSpec;
    type SpecGenerator = PuzzleSpecGenerator;

    const NAME: &str = "puzzle";

    fn get_spec_filename(spec: &Self::Spec) -> Option<String> {
        spec.meta.tags.filename().map(str::to_owned)
    }
    fn get_spec_authors(spec: &Self::Spec) -> impl IntoIterator<Item = &String> {
        spec.meta.tags.authors()
    }
    fn get_generator_filename(generator: &Self::SpecGenerator) -> Option<String> {
        generator.meta.tags.filename().map(str::to_owned)
    }
    fn get_generator_examples(
        generator: &Self::SpecGenerator,
    ) -> Option<&HashMap<String, Arc<Self::Spec>>> {
        Some(&generator.examples)
    }
    fn get_generator_authors(generator: &Self::SpecGenerator) -> impl IntoIterator<Item = &String> {
        generator.meta.tags.authors()
    }

    fn build_object_from_spec(ctx: BuildCtx, spec: &Arc<Self::Spec>) -> BuildResult<Self, String> {
        (spec.build)(ctx).map_err(|e| format!("{e:#}"))
    }
    fn generate_spec(
        ctx: BuildCtx,
        generator: &Arc<Self::SpecGenerator>,
        params: Vec<String>,
    ) -> BuildResult<Self::Spec, String> {
        (generator.generate)(ctx, params).map_err(|e| format!("{e:#}"))
    }
}

impl CatalogObject for ColorSystem {
    fn get_subcatalog(db: &Db) -> &SubCatalog<Self> {
        &db.color_systems
    }
    fn get_subcatalog_mut(db: &mut Db) -> &mut SubCatalog<Self> {
        &mut db.color_systems
    }
}
impl CatalogObjectImpl for ColorSystem {
    type Spec = ColorSystem;
    type SpecGenerator = ColorSystemGenerator;

    const NAME: &str = "color system";

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

    fn build_object_from_spec(_ctx: BuildCtx, spec: &Arc<Self::Spec>) -> BuildResult<Self, String> {
        Ok(Redirectable::Direct(Arc::clone(spec)))
    }
    fn generate_spec(
        ctx: BuildCtx,
        generator: &Arc<Self::SpecGenerator>,
        params: Vec<String>,
    ) -> BuildResult<Self::Spec, String> {
        (generator.generate)(ctx, params).map_err(|e| format!("{e:#}"))
    }
}

impl CatalogObject for TwistSystem {
    fn get_subcatalog(db: &Db) -> &SubCatalog<Self> {
        &db.twist_systems
    }
    fn get_subcatalog_mut(db: &mut Db) -> &mut SubCatalog<Self> {
        &mut db.twist_systems
    }
}
impl CatalogObjectImpl for TwistSystem {
    type Spec = TwistSystemSpec;
    type SpecGenerator = TwistSystemSpecGenerator;

    const NAME: &str = "twist system";

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

    fn build_object_from_spec(ctx: BuildCtx, spec: &Arc<Self::Spec>) -> BuildResult<Self, String> {
        (spec.build)(ctx).map_err(|e| format!("{e:#}"))
    }
    fn generate_spec(
        ctx: BuildCtx,
        generator: &Arc<Self::SpecGenerator>,
        params: Vec<String>,
    ) -> BuildResult<Self::Spec, String> {
        (generator.generate)(ctx, params).map_err(|e| format!("{e:#}"))
    }
}
