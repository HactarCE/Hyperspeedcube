use std::fmt;
use std::io::Write;

use hyperpuzzle_core::Catalog;

mod hps_construction;
mod verification;

fn load_new_catalog() -> Catalog {
    let catalog = Catalog::new();
    time_it("Loading all puzzles", || {
        crate::load_catalog(&catalog);
    });
    catalog
}

fn time_it<T>(task: impl fmt::Display, f: impl FnOnce() -> T) -> (T, std::time::Duration) {
    print!("{task} ...");
    std::io::stdout().flush().expect("error flushing stdout");
    let t1 = std::time::Instant::now();
    let ret = f();
    let elapsed = t1.elapsed();
    println!(" done in {elapsed:?}");
    (ret, elapsed)
}
