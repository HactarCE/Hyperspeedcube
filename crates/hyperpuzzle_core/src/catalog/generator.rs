use log::Level;

use super::*;
use crate::LogLine;

/// Context when building an object in the catalog.
///
/// This type is reference-counted and thus cheap to clone.
#[derive(Clone)]
pub struct BuildCtx(pub(super) Arc<BuildCtxInner>);

pub(super) struct BuildCtxInner {
    /// Catalog.
    pub(super) catalog: Catalog,
    /// ID of the object being built.
    pub(super) id: CatalogId,

    /// Mutable request state.
    ///
    /// We reference [`GenericRequestState`] instead of [`GenericRequestData`]
    /// to avoid keeping the request alive if all `Request`s are dropped.
    pub(super) request_state: Arc<Mutex<GenericRequestState>>,
    /// Whether all requests to build the object have been dropped.
    pub(super) canceled: Arc<AtomicBool>,
    /// List of parent IDs.
    ///
    /// If this list exceeds [`crate::MAX_ID_REDIRECTS`] or if there is an
    /// attempt to create a cycle, then an error is returned.
    pub(super) parent_ids: Vec<String>,
}

impl BuildCtx {
    /// Returns the ID of the requested object.
    ///
    /// It is ok to return an object with a different ID only if the object has
    /// been built using the new ID.
    pub fn id(&self) -> &CatalogId {
        &self.0.id
    }

    pub fn logger(&self) -> &Logger {
        &self.0.catalog.logger
    }

    /// Returns whether all requests to build the object have been dropped.
    pub fn is_canceled(&self) -> bool {
        self.0.canceled.load(Ordering::Relaxed)
    }

    /// Returns an error if all requests to build the object have been dropped.
    pub fn cancel_if_unrequested(&self) -> Result<()> {
        if self.is_canceled() {
            bail!("canceled")
        } else {
            Ok(())
        }
    }

    pub fn warn_fn(&self) -> impl Fn(eyre::Report) {
        let l = self.0.catalog.logger.clone();
        move |e| {
            l.log(LogLine {
                level: Level::Warn,
                filename: None,
                msg: format!("{e}"),
                full: Some(format!("{e:?}")), // include backtrace
            });
        }
    }

    /// Adds a task to the task stack.
    ///
    /// Each call to [`BuildCtx::push_task()`] should have a matching call to
    /// [`BuildCtx::pop_task()`].
    ///
    /// Prefer a lowercased verb phrase as the description; e.g., `attaching
    /// knobs` rather than `Attaching knobs`.
    pub fn push_task(&self, description: impl ToString) {
        self.0
            .request_state
            .lock()
            .task_stack
            .push(description.to_string());
    }
    /// Removes a task from the task stack.
    ///
    /// Each call to [`BuildCtx::pop_task()`] should have a matching call to
    /// [`BuildCtx::push_task()`]; however, [`BuildCtx::pop_task()`] should
    /// _not_ be called in the event that an error occurs during a task.
    pub fn pop_task(&self) {
        self.0.request_state.lock().task_stack.pop();
    }

    /// Builds a catalog object by ID.
    ///
    /// This is a wrapper around [`Catalog::build_blocking()`] that ensures
    /// cancellations propagate to the subrequest.
    pub fn build_blocking<T: CatalogObject>(&self, id: &CatalogId) -> Result<Arc<T>> {
        self.cancel_if_unrequested()?;

        let (new_request, call_if_new) = self.0.catalog.new_request(id, &self.0.parent_ids);

        match new_request.inner {
            RequestInner::Precomputed(result) => Ok(result.clone()?),
            RequestInner::Requested {
                generic_request, ..
            } => {
                let waiter = generic_request.data.waiter.clone();

                // Move the subrequest into `self.0.request_data` so that it is
                // dropped if the original request is dropped.
                let mut request_data_guard = self.0.request_state.lock();
                let subrequest = &mut request_data_guard.subrequest;
                if subrequest.is_some() {
                    bail!("parallel subrequests are not allowed");
                }
                *subrequest = Some(generic_request);
                drop(request_data_guard);

                // Build the object and/or wait until it has been built.
                if let Some(f) = call_if_new {
                    f();
                }
                waiter.wait();

                // Remove the subrequest.
                let mut request_data_guard = self.0.request_state.lock();
                let subrequest = &mut request_data_guard.subrequest;
                *subrequest = None;
                drop(request_data_guard);

                // The object should already be built.
                Ok(self.0.catalog.build_blocking(id)?)
            }
        }
    }

    /// Parses an ID string and builds the corresponding catalog object.
    ///
    ///
    /// This is a wrapper around [`Catalog::build_blocking()`] that parses an ID
    /// from a string and ensures cancellations propagate to the subrequest.
    pub fn build_str_blocking<T: CatalogObject>(&self, id_str: &str) -> Result<Arc<T>> {
        let type_name = T::catalog_type_name();
        self.build_blocking(
            &id_str
                .parse()
                .map_err(|e| eyre!("error parsing {type_name} ID string {id_str:?}: {e}"))?,
        )
    }

    /// Builds the corresponding catalog objects for a list of IDs specified in
    /// a single argument.
    pub fn build_list_blocking<T: CatalogObject>(
        &self,
        arg: &CatalogIdValue,
    ) -> Result<Vec<Arc<T>>> {
        arg.clone()
            .into_list()?
            .into_iter()
            .map(|val| self.build_blocking(&val.into_id()?))
            .collect()
    }

    pub(super) fn store_result<T: CatalogObject>(self, result: CatalogResult<T>) {
        let catalog = &self.0.catalog;
        let subcatalog = catalog.get_subcatalog::<T>().expect("missing subcatalog");
        let mut cache = subcatalog.cache.lock();

        // Bail if canceled because the task may have already been restarted.
        // It's important to do this while the subcatalog cache mutex is locked
        // to avoid race conditions.
        if self.cancel_if_unrequested().is_err() {
            log::trace!("task canceled; discarding result");
            return; // silently failure is ok
        }

        // Ensure that ID of constructed object matches ID of request
        if let Ok(obj) = &result {
            if self.id() == obj.id() {
                // ID of constructed object matches ID of request. ok.
            } else if let Some(other_cache_entry) = cache.get(&obj.id().to_string())
                && let CacheEntry::Done(other_result) = other_cache_entry
                && let Ok(other_obj) = other_result
                && Arc::ptr_eq(other_obj, obj)
            {
                // ID of constructed object is different from ID of request, but
                // object matches cache entry with constructed ID. i.e., request
                // got redirected to a different ID. ok.
            } else {
                // ID does not match and is not an ID redirect.
                catalog.logger.warn(format!(
                    "{} `{}` has ID `{}` when built",
                    T::catalog_type_name(),
                    self.id(),
                    obj.id(),
                ));
            }
        }

        cache.insert(self.0.id.to_string(), CacheEntry::Done(result));
    }
}

/// Object generator.
pub struct Generator<T> {
    /// Catalog ID without any parameters or subset.
    pub id: CatalogIdent,
    /// Parameter types, ranges, and defaults.
    pub params: Vec<GeneratorParam>,
    /// Subset parameter, if any.
    pub subset_param: Option<GeneratorSubsetParam>,

    /// Options when validating parameters.
    pub validation: GeneratorParamValidation,

    /// Function to generate metadata for the object from parameters.
    ///
    /// **This may be expensive. Do not call it from UI thread.**
    ///
    /// Generator parameters and subset can be retrieved from the provided
    /// [`BuildCtx::id`]. When building an object that supports subsets using
    /// [`Catalog::build()`] or a similar method, the subset will _always_ be
    /// included in [`BuildCtx::id`], even if it was omitted when requesting a
    /// catalog object.
    pub generate: GenerateFn<T>,
}

impl<T> fmt::Debug for Generator<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct(&format!("Generator<{}>", std::any::type_name::<T>()))
            .field("id", &self.id)
            .field("params", &self.params)
            .finish_non_exhaustive()
    }
}

impl<T: CatalogObject> fmt::Display for Generator<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} generator `{}`", T::catalog_type_name(), self.id)
    }
}

impl<T: CatalogObject> Generator<T> {
    /// Constructs a generator that takes no parameters.
    pub fn new_constant(
        id: CatalogIdent,
        generate: impl 'static + Send + Sync + Fn(BuildCtx) -> Result<Arc<T>>,
    ) -> Self {
        Self {
            id,
            params: vec![],
            subset_param: None,
            // validation doesn't matter; always empty
            validation: GeneratorParamValidation { allow_empty: true },
            generate: Box::new(generate),
        }
    }

    /// Returns an error if the ID does not match the generator parameters.
    ///
    /// This catches most common error. Integer bounds are not checked.
    pub fn validate(&self, id: &CatalogId) -> Result<()> {
        // Check subset
        if let Some(subset) = &id.subset {
            let Some(subset_param) = &self.subset_param else {
                bail!("{self} has no subset parameter; got `{subset}`");
            };
            if !subset_param
                .options
                .iter()
                .any(|option| &option.id == subset)
            {
                bail!(
                    "{self} has no `{subset}` subset, only {}",
                    subset_param
                        .options
                        .iter()
                        .map(|option| format!("`{}`", option.id))
                        .join(", "),
                );
            }
        }

        // Check arguments
        if id.args.is_empty() && self.validation.allow_empty {
            // ok
        } else {
            let expected = self.params.len();
            let got = id.args.len();
            if expected != got {
                bail!("{self} requires {expected} params; got {got}");
            }
            for (i, (param, arg)) in std::iter::zip(&self.params, &id.args).enumerate() {
                param
                    .typed_value(arg.clone())
                    .with_context(|| format!("bad value for param at index {i} for {self}"))?;
            }
        }

        Ok(())
    }

    /// Canonicalizes the ID, or returns `None` if the ID is already canonical.
    ///
    /// This method is idempotent.
    #[must_use]
    pub fn canonicalize(&self, id: &CatalogId) -> Option<CatalogId> {
        // Remove subset if default
        if let Some(subset_param) = &self.subset_param
            && id.subset.is_some()
            && id.subset == subset_param.default
        {
            Some(CatalogId {
                base: id.base.clone(),
                args: id.args.clone(),
                subset: None,
            })
        } else {
            None
        }
    }

    /// Returns the ID of some default value for this generator.
    pub fn default_id(&self) -> CatalogId {
        CatalogId {
            base: self.id.clone(),
            args: self.params.iter().map(|p| p.default.clone()).collect(),
            subset: match &self.subset_param {
                Some(subsets) => match &subsets.default {
                    Some(default_id) => Some(default_id.clone()),
                    None => subsets.options.first().map(|option| option.id.clone()),
                },
                None => None,
            },
        }
    }
}

/// Type of [`Generator::generate`].
pub type GenerateFn<T> = Box<dyn Send + Sync + Fn(BuildCtx) -> Result<Arc<T>>>;

/// Subset parameter for a [`Generator`].
#[derive(Debug, Clone)]
pub struct GeneratorSubsetParam {
    /// Available subsets.
    ///
    /// Most of the time, either this list is empty or the only options are
    /// `rot` and `refl` (for a rotational or reflection puzzle, respectively).
    pub options: Vec<GeneratorSubsetParamValue>,
    /// Default subset to use when constructing the object, in case the ID does
    /// not specify.
    pub default: Option<CatalogIdent>,
    /// Maximal subset, if there is an unambiguous answer. This is sometimes
    /// used as the default subset, such as when constructing one factor of a
    /// product puzzle.
    pub maximal: Option<CatalogIdent>,
}

/// Allowed value for the subset parameter of a [`Generator`].
#[derive(Debug, Clone)]
pub struct GeneratorSubsetParamValue {
    /// ID suffix for the subset. Typically `rot` or `refle`.
    pub id: CatalogIdent,
    /// Word to prepend to the name, with a space. Typically `Rot ` or `Refle `.
    pub name_prefix: String,
}

/// Validation options for a generator.
#[derive(Debug, Clone)]
pub struct GeneratorParamValidation {
    /// Whether to allow an empty parameter list.
    ///
    /// This can be used to generate metadata for a generator itself.
    pub allow_empty: bool,
}
