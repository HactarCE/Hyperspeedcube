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
}

impl BuildCtx {
    pub(super) fn new(catalog: &Catalog, progress: &Arc<Mutex<Progress>>) -> Self {
        Self {
            catalog: catalog.clone(),
            progress: Arc::clone(progress),
        }
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
    pub fn new_constant(object: Arc<T>) -> Self {
        Self::new_lazy_constant(Arc::clone(object.meta()), move |_| {
            Ok(Redirectable::Direct(Arc::clone(&object)))
        })
    }

    /// Constructs a generator that takes no parameters and has a constant
    /// output that is lazily-constructed.
    pub fn new_lazy_constant(
        meta: Arc<CatalogMetadata>,
        f: impl 'static + Send + Sync + Fn(BuildCtx) -> Result<Redirectable<Arc<T>>>,
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
    pub fn try_map<U, E>(self, f: impl FnOnce(T) -> Result<U, E>) -> Result<Redirectable<U>, E> {
        Ok(match self {
            Redirectable::Direct(inner) => Redirectable::Direct(f(inner)?),
            Redirectable::Redirect(id) => Redirectable::Redirect(id),
        })
    }
}
