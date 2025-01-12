use std::fmt;
use std::io::Write;

use hyperpuzzle::Library;

mod lua_construction;
mod verification;

fn load_puzzle_library() -> Library {
    let lib = Library::new();
    time_it("Loading all puzzles", || crate::load_puzzles_in_lib(&lib));
    lib
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
