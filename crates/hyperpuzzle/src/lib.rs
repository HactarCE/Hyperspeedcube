//! Catalog of all official puzzles for Hyperspeedcube.
//!
//! For convenience, this crate also re-exports all of `hyperpuzzle_core`.
//!
//! # Example
//!
//! ```rust
//! hyperpuzzle::load_global_catalog();
//!
//! let puzzle = hyperpuzzle::catalog().build_puzzle_blocking("ft_cube:3").unwrap();
//! assert_eq!("3x3x3", puzzle.meta.name);
//!
//! let puzzle = hyperpuzzle::catalog().build_puzzle_blocking("ft_cube:2").unwrap();
//! assert_eq!("2x2x2", puzzle.meta.name);
//! ```
//!
//! # Dynamically loading Lua files
//!
//! By default, the built-in files are baked into the executable. To dynamically
//! load Lua files, add this to your `Cargo.toml` and build with the environment
//! variable `HSC_OFFICIAL_BUILD=1`:
//!
//! ```toml
//! hyperpuzzle_lua = { version = "*", features = ["hyperpaths"] }
//! ```

use std::sync::Arc;

pub use hyperpuzzle_core::*;
use lazy_static::lazy_static;
use parking_lot::Mutex;
pub use prelude::*;
pub use {hyperpuzzle_core as core, hyperpuzzle_impl_nd_euclid as nd_euclid};

/// Prelude of common imports.
pub mod prelude {
    pub use hyperpuzzle_core::prelude::*;
    pub use hyperpuzzle_impl_nd_euclid::*;
}

lazy_static! {
    /// Even though `Catalog` already contains an `Arc<Mutex<T>>` internally, we
    /// use another layer of `Arc<Mutex<Catalog>>` here so that we can reset the
    /// catalog without interfering with old references to it.
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
    hyperpuzzle_lua::load_puzzles(catalog, catalog.default_logger());
}

#[cfg(test)]
mod tests;
