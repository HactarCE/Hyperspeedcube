use super::*;

/// Subcatalog for a specific object type (puzzles, color systems, twist
/// systems, etc.).
pub struct SubCatalog<T> {
    /// Object generators, indexed by generator ID (e.g., `ft_cube`).
    ///
    /// This includes non-generated objects, which are equivalent to generators
    /// that take no parameters.
    pub generators: HashMap<String, Arc<Generator<T>>>,
    /// Cache of objects created from generators, indexed by ID (e.g.,
    /// `ft_cube(3)`).
    pub cache: Mutex<HashMap<String, Arc<Mutex<CacheEntry<T>>>>>,
}

impl<T> fmt::Debug for SubCatalog<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SubCatalog")
            .field("generators", &self.generators.keys().collect_vec())
            .finish_non_exhaustive()
    }
}

impl<T> Default for SubCatalog<T> {
    fn default() -> Self {
        Self {
            generators: HashMap::default(),
            cache: Mutex::default(),
        }
    }
}

impl<T> SubCatalog<T> {
    /// Adds a generator to the catalog.
    pub(super) fn add(&mut self, generator: Arc<Generator<T>>) -> Result<()> {
        if !generator.meta.id.args.is_empty() {
            bail!("object ID cannot have arguments")
        }
        match self.generators.entry(generator.meta.id.to_string()) {
            hash_map::Entry::Occupied(occupied_entry) => {
                bail!("duplicate ID {:?}", occupied_entry.key())
            }
            hash_map::Entry::Vacant(vacant_entry) => {
                vacant_entry.insert(generator);
                Ok(())
            }
        }
    }

    /// Returns the cache entry for an ID.
    pub(super) fn cache_entry(&self, id: &CatalogId) -> Arc<Mutex<CacheEntry<T>>> {
        Arc::clone(self.cache.lock().entry(id.to_string()).or_default())
    }

    pub(super) fn remove_cache_entry(&self, id: &CatalogId) {
        self.cache.lock().remove(&id.to_string());
    }
}
