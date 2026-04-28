use crate::ComponentList;

use super::*;

/// Puzzle generator.
pub type PuzzleGenerator = Generator<Puzzle>;
/// Color system generator.
pub type ColorSystemGenerator = Generator<ColorSystem>;
/// Twist system generator.
pub type TwistSystemGenerator = Generator<TwistSystem>;

/// Context when building an object in the catalog.
#[derive(Clone)]
pub struct BuildCtx {
    /// Catalog.
    pub catalog: Catalog,
    /// Progress output.
    pub progress: Arc<Mutex<Progress>>,
    /// ID of the object being built.
    pub id: CatalogId,
}

impl BuildCtx {
    pub(super) fn new(catalog: &Catalog, progress: &Arc<Mutex<Progress>>, id: CatalogId) -> Self {
        Self {
            catalog: catalog.clone(),
            progress: Arc::clone(progress),
            id,
        }
    }

    /// Builds a catalog object by ID.
    ///
    /// This is a wrapper around [`Catalog::build_blocking()`] that sets
    /// [`Progress::task`] temporarily.
    pub fn build_blocking<T: CatalogObject>(&self, id: &CatalogId) -> Result<Arc<T>> {
        let type_name = T::CATALOG_TYPE_NAME;
        let old_task = std::mem::replace(
            &mut self.progress.lock().task,
            BuildTask::Building(type_name),
        );
        let result = self
            .catalog
            .build_blocking(id)
            .map_err(|e| eyre!("{e:?}")) // include backtrace
            .wrap_err_with(|| format!("error building {type_name}"));
        self.progress.lock().task = old_task;
        result
    }

    /// Parses an ID string and builds the corresponding catalog object.
    ///
    /// This is a wrapper around [`Catalog::build_blocking()`] that parses an ID
    /// from a string and sets [`Progress::task`] temporarily.
    pub fn build_str_blocking<T: CatalogObject>(&self, id_str: &str) -> Result<Arc<T>> {
        let type_name = T::CATALOG_TYPE_NAME;
        self.build_blocking(
            &id_str
                .parse()
                .map_err(|e| eyre!("error parsing {type_name} ID string {id_str:?}: {e}"))?,
        )
    }

    /// Sets the current task to
    /// [`BuildTask::Building`]`(T::CATALOG_TYPE_NAME)`.
    pub fn set_building<T: CatalogObject>(&self) {
        self.set_task(BuildTask::Building(T::CATALOG_TYPE_NAME));
    }

    /// Sets the current task.
    pub fn set_task(&self, new_task: BuildTask) {
        self.progress.lock().task = new_task;
    }
}

/// Object generator.
pub struct Generator<T> {
    /// Metadata.
    pub meta: Arc<CatalogMetadata>,
    /// Parameter types, ranges, and defaults.
    pub params: Vec<GeneratorParam>,
    /// Function to generate metadata for the object from parameters.
    ///
    /// **This may be expensive. Do not call it from UI thread.**
    pub generate_meta: GenerateFn<CatalogMetadata>,
    /// Function to generate the object from parameters.
    ///
    /// **This may be expensive. Do not call it from UI thread.**
    pub generate: GenerateFn<T>,
    /// Optional backend-specific data.
    pub components: ComponentList<Self>,
}

impl<T> fmt::Debug for Generator<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct(&format!("Generator<{}>", std::any::type_name::<T>()))
            .field("meta", &self.meta)
            .field("params", &self.params)
            .finish_non_exhaustive()
    }
}

impl<T: CatalogObject> Generator<T> {
    /// Constructs a generator that takes no parameters and has a constant
    /// output.
    pub fn new_constant(object: Arc<T>, components: ComponentList<Generator<T>>) -> Self {
        Self::new_lazy_constant(
            Arc::clone(object.meta()),
            move |_| Ok(Redirectable::Direct(Arc::clone(&object))),
            components,
        )
    }

    /// Constructs a generator that takes no parameters and has a constant
    /// output that is lazily-constructed.
    pub fn new_lazy_constant(
        meta: Arc<CatalogMetadata>,
        f: impl 'static + Send + Sync + Fn(BuildCtx) -> Result<Redirectable<Arc<T>>>,
        components: ComponentList<Generator<T>>,
    ) -> Self {
        let meta2 = Arc::clone(&meta);
        Self {
            meta: Arc::clone(&meta),
            params: vec![],
            generate_meta: Box::new(move |_, args| {
                ensure!(args.is_empty(), "{} is not a generator", meta.id);
                Ok(Redirectable::Direct(Arc::clone(&meta)))
            }),
            generate: Box::new(move |build_ctx, args| {
                ensure!(args.is_empty(), "{} is not a generator", meta2.id);
                f(build_ctx)
            }),
            components,
        }
    }

    /// Returns the ID of the default for this generator.
    pub fn default_id(&self) -> CatalogId {
        CatalogId {
            base: self.meta.id.base.clone(),
            args: self
                .params
                .iter()
                .map(|p| p.default.clone().into())
                .collect(),
        }
    }
}

/// Type of [`Generator::generate`].
pub type GenerateFn<T> =
    Box<dyn Send + Sync + Fn(BuildCtx, Vec<CatalogArgValue>) -> Result<Redirectable<Arc<T>>>>;

/// Possible ID redirect.
#[derive(Debug, Clone)]
pub enum Redirectable<T> {
    /// Object directly generated.
    Direct(T),
    /// Redirect to a different ID.
    Redirect(String),
}

impl<T> Redirectable<T> {
    /// Applies a function to the contained `T`.
    pub fn and_then<U, E>(
        self,
        f: impl FnOnce(T) -> Result<Redirectable<U>, E>,
    ) -> Result<Redirectable<U>, E> {
        match self {
            Redirectable::Direct(inner) => f(inner),
            Redirectable::Redirect(id) => Ok(Redirectable::Redirect(id)),
        }
    }
}
