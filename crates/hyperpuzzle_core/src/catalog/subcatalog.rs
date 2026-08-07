use super::*;

/// Subcatalog for a specific object type (puzzles, color systems, twist
/// systems, etc.).
pub struct SubCatalog<T> {
    /// Object generators, indexed by generator ID (e.g., `ft_cube`).
    ///
    /// This includes non-generated objects, which are equivalent to generators
    /// that take no parameters.
    pub generators: HashMap<String, Arc<Generator<T>>>,
    /// Hooks, which are called on the output of generators before they are
    /// cached and returned.
    ///
    /// Hooks are sorted by priority.
    pub hooks: Vec<Arc<CatalogHook<T>>>,
    /// Cache of objects created from generators, indexed by ID (e.g.,
    /// `ft_cube(3)`).
    pub(super) cache: Mutex<HashMap<String, CacheEntry<T>>>,
}

impl<T> fmt::Debug for SubCatalog<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SubCatalog")
            .field("generators", &self.generators.keys().collect_vec())
            .finish_non_exhaustive()
    }
}

impl<T: CatalogObject> Default for SubCatalog<T> {
    fn default() -> Self {
        Self {
            generators: HashMap::default(),
            hooks: vec![],
            cache: Mutex::default(),
        }
    }
}

impl<T: CatalogObject> SubCatalog<T> {
    /// Adds a generator to the catalog.
    pub(super) fn add(&mut self, generator: Arc<Generator<T>>) -> Result<()> {
        match self.generators.entry(generator.id.to_string()) {
            hash_map::Entry::Occupied(occupied_entry) => {
                bail!("duplicate ID {:?}", occupied_entry.key())
            }
            hash_map::Entry::Vacant(vacant_entry) => {
                vacant_entry.insert(generator);
                Ok(())
            }
        }
    }

    /// Adds a hook to the catalog.
    pub(super) fn add_hook(&mut self, hook: Arc<CatalogHook<T>>) {
        self.hooks.push(hook);

        // Sorting every time ends up being O(n^2) with respect to number of
        // hooks but this is probably fine. Emit a warning if there seems to be
        // a lot of hooks.
        self.hooks.sort_by_key(|h| h.priority);
        if self.hooks.len() == 100 {
            log::warn!(
                "{} subcatalog has 100+ hooks! this may cause performance problems",
                T::catalog_type_name(),
            )
        }
    }

    pub(super) fn try_get_generator(&self, id_base: &str) -> Result<&Arc<Generator<T>>> {
        self.generators.get(id_base).ok_or_else(|| {
            eyre!(
                "no {ty} or {ty} generator with ID {id_base:?}",
                ty = T::catalog_type_name(),
            )
        })
    }

    /// Fetches the cache entry for an ID, creating one if it is missing.
    /// Returns a request for the object and a boolean indicating whether the
    /// request is new (and thus the caller is responsible for actually building
    /// the object).
    ///
    /// If the request for the object is dropped while the being built, and
    /// there are no other requests, then building the object is canceled.
    pub(super) fn request_cache_entry(
        &self,
        catalog: &Catalog,
        id: CatalogId,
    ) -> (Request<T>, bool) {
        if *id.base == *crate::AD_HOC_ID_STR {
            return (
                Request::new_error(
                    &id,
                    eyre!(
                        "ad-hoc generator cannot be called directly \
                         (if you are seeing this, it's probably a bug)"
                    ),
                ),
                false,
            );
        }

        let mut cache_guard = self.cache.lock();

        let cache_entry = cache_guard.entry(id.to_string());
        let is_new = matches!(cache_entry, hash_map::Entry::Vacant(_));
        let request_inner = match cache_entry {
            hash_map::Entry::Occupied(e) => match e.get() {
                CacheEntry::Building { request_data, .. } => RequestInner::Requested {
                    catalog: catalog.clone(),
                    generic_request: GenericRequest {
                        data: Arc::clone(request_data),
                    },
                },
                CacheEntry::Done(result) => RequestInner::Precomputed(result.clone()),
            },
            hash_map::Entry::Vacant(e) => {
                let notify = NotifyWhenDropped::new();
                let request_data =
                    GenericRequestData::new::<T>(catalog.clone(), id.clone(), notify.waiter());

                e.insert(CacheEntry::Building {
                    request_data: Arc::clone(&request_data),
                    _notify: notify,
                });

                RequestInner::Requested {
                    catalog: catalog.clone(),
                    generic_request: GenericRequest { data: request_data },
                }
            }
        };

        (
            Request {
                requested_id: id,
                inner: request_inner,
            },
            is_new,
        )
    }
}
