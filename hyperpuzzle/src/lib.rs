use std::sync::Arc;

use hyperpuzzle_core::Catalog;
use lazy_static::lazy_static;
use parking_lot::Mutex;

lazy_static! {
    /// Even though `Catalog` already contains an `Arc<Mutex<T>>` internally, we
    /// use another layer of `Arc<Mutex<Catalog>>` here so that we can reset the
    /// catalog without old interfering with old references to it.
    static ref CATALOG: Arc<Mutex<Catalog>> = Arc::new(Mutex::new(Catalog::new()));
}

/// Returns the global catalog.
pub fn catalog() -> Catalog {
    CATALOG.lock().clone()
}

/// Reloads all puzzle backends into the global catalog and clears the cache.
pub fn load_global_catalog() {
    let mut catalog = CATALOG.lock();
    *catalog = Catalog::new();

    load_catalog(&catalog);
}

/// Loads all puzzle backends into a catalog.
pub fn load_catalog(catalog: &Catalog) {
    hyperpuzzle_lua::load_puzzles(&catalog, catalog.logger());
}

#[cfg(test)]
mod tests;
