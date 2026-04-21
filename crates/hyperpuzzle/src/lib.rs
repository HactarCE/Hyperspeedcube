//! Catalog of all official puzzles for Hyperspeedcube.
//!
//! For convenience, this crate also re-exports all of `hyperpuzzle_core`.
//!
//! # Example
//!
//! ```rust
//! # use std::sync::Arc;
//! # use hyperpuzzle::{CatalogId, Puzzle};
//! hyperpuzzle::load_global_catalog();
//!
//! let id: CatalogId = "ft_cube(3)".parse().unwrap();
//! let puzzle: Arc<Puzzle> = hyperpuzzle::catalog().build_blocking(id).unwrap();
//! assert_eq!("3x3x3", puzzle.meta.name);
//!
//! let id: CatalogId = "ft_cube(2)".parse().unwrap();
//! let puzzle: Arc<Puzzle> = hyperpuzzle::catalog().build_blocking(id).unwrap();
//! assert_eq!("2x2x2", puzzle.meta.name);
//! ```
//!
//! # Dynamically loading Hyperpuzzlescript files
//!
//! By default, the built-in files are baked in. To load them at runtime, add
//! this to your `Cargo.toml`:
//!
//! ```toml
//! hyperpuzzlescript = { version = "*", features = ["hyperpaths"] }
//! ```

use std::sync::Arc;

pub use hyperpuzzle_core as core;
pub use hyperpuzzle_core::*;
pub use hyperpuzzle_impl_nd_euclid as nd_euclid;
pub use hyperpuzzle_impl_symmetric as symmetric;
use lazy_static::lazy_static;
use parking_lot::Mutex;
pub use prelude::*;

/// Prelude of common imports.
pub mod prelude {
    pub use hyperpuzzle_core::prelude::*;
    pub use hyperpuzzle_impl_nd_euclid::prelude::*;
}

lazy_static! {
    /// Global catalog.
    ///
    /// Even though [`Catalog`] already contains an `Arc<Mutex<T>>` internally,
    /// we use another layer of `Arc<Mutex<Catalog>>` here so that we can reset
    /// the catalog without interfering with old references to it.
    static ref CATALOG: Arc<Mutex<Catalog>> = Arc::new(Mutex::new(Catalog::default()));
}

/// Returns the global catalog.
pub fn catalog() -> Catalog {
    CATALOG.lock().clone()
}

/// Reloads all puzzle backends into the global catalog and clears the cache.
pub fn load_global_catalog() {
    let new_catalog = CatalogBuilder::default();
    load_catalog(&new_catalog).expect("error loading catalog");
    *CATALOG.lock() = new_catalog.build().expect("error building catalog");
}

/// Loads all puzzle backends into a catalog.
pub fn load_catalog(catalog: &CatalogBuilder) -> eyre::Result<()> {
    let mut rt = hyperpuzzlescript::Runtime::new();

    let logger = catalog.logger()?;
    rt.on_print = Box::new(move |msg| {
        logger.log(LogLine {
            level: Level::Info,
            msg,
            full: None,
        });
    });

    let logger = catalog.logger()?;
    rt.on_diagnostic = Box::new(move |modules, diagnostic| {
        logger.log(LogLine {
            level: match diagnostic.msg {
                hyperpuzzlescript::Diagnostic::Error(_) => Level::Error,
                hyperpuzzlescript::Diagnostic::Warning(_) => Level::Warn,
            },
            msg: diagnostic.msg.to_string(),
            full: Some(diagnostic.formatted(modules).ansi_string),
        });
    });

    let (eval_tx, eval_rx) = hyperpuzzlescript::EvalRequestTx::new();

    // Add base built-ins.
    rt.with_builtins(hyperpuzzlescript::builtins::define_base_in)
        .expect("error defining HPS built-ins");

    // Add catalog built-ins.
    rt.with_builtins(|builtins| {
        hyperpuzzlescript::builtins::catalog::define_in(builtins, catalog, &eval_tx)
    })
    .expect("error defining HPS catalog built-ins");

    // Add NdEuclid built-ins.
    rt.register_puzzle_engine(Arc::new(hyperpuzzle_impl_nd_euclid::hps::HpsNdEuclid));
    rt.register_twist_system_engine(Arc::new(hyperpuzzle_impl_nd_euclid::hps::HpsNdEuclid));
    rt.with_builtins(hyperpuzzle_impl_nd_euclid::hps::define_in)
        .expect("error defining HPS euclid built-ins");

    rt.with_builtins(|builtins| hyperpuzzle_impl_symmetric::hps::define_in(builtins, catalog))
        .expect("error defining HPS symmetric built-ins");
    hyperpuzzle_impl_symmetric::add_puzzles_to_catalog(catalog)
        .expect("error adding symmetric puzzles to catalog");

    // Load user files.
    rt.modules.add_default_files();
    rt.exec_all_files();
    std::thread::spawn(move || {
        for eval_request in eval_rx {
            eval_request(&mut rt);
        }
    });

    Ok(())
}

#[cfg(test)]
mod tests;
