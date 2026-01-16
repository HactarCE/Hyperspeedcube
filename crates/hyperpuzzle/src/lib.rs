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
//! # Dynamically loading Hyperpuzzlescript files
//!
//! By default, the built-in files are baked in. To load them at runtime, add
//! this to your `Cargo.toml`:
//!
//! ```toml
//! hyperpuzzlescript = { version = "*", features = ["hyperpaths"] }
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
    pub use hyperpuzzle_impl_nd_euclid::prelude::*;
}

lazy_static! {
    /// Global catalog.
    ///
    /// Even though [`Catalog`] already contains an `Arc<Mutex<T>>` internally,
    /// we use another layer of `Arc<Mutex<Catalog>>` here so that we can reset
    /// the catalog without interfering with old references to it.
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
    let mut rt = hyperpuzzlescript::Runtime::new();

    let logger = catalog.default_logger().clone();
    // rt.on_print = Box::new(move |msg| {
    //     logger.log(LogLine {
    //         level: Level::Info,
    //         msg,
    //         full: None,
    //     });
    // });

    let logger = catalog.default_logger().clone();
    // rt.on_diagnostic = Box::new(move |modules, diagnostic| {
    //     logger.log(LogLine {
    //         level: match diagnostic.msg {
    //             hyperpuzzlescript::Diagnostic::Error(_) => Level::Error,
    //             hyperpuzzlescript::Diagnostic::Warning(_) => Level::Warn,
    //         },
    //         msg: diagnostic.msg.to_string(),
    //         full: Some(diagnostic.to_string(modules)),
    //     });
    // });

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

    // Load user files.
    rt.modules.add_default_files();
    rt.exec_all_files();
    std::thread::spawn(move || {
        let mut i = 0;
        for eval_request in eval_rx {
            dbg!("received eval req", i);
            eval_request(&mut rt);
            dbg!("completed eval req", i);
            i += 1;
        }
    });
}

#[cfg(test)]
mod tests;
