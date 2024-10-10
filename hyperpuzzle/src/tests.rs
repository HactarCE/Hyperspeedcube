use std::{fmt, io::Write, path::Path};

use eyre::Result;

use crate::*;

#[test]
fn load_all_puzzle_definitions() {
    load_puzzle_library();
}

#[test]
fn lint_all_puzzle_definitions() -> Result<(), ()> {
    let lib = load_puzzle_library();

    let mut any_failed = false;

    for puzzle in lib.puzzles() {
        let puzzle_lint_output = time_it(format!("Linting puzzle {}", puzzle.id), || {
            PuzzleLintOutput::from_spec(puzzle)
        });
        if !puzzle_lint_output.all_good() {
            any_failed = true;

            let PuzzleLintOutput {
                puzzle: _,
                missing_tags,
            } = puzzle_lint_output;

            if !missing_tags.is_empty() {
                println!("  Missing tags:");
                for tag in missing_tags {
                    println!("    {tag:?}")
                }
            }
        }
    }

    match any_failed {
        true => Err(()),
        false => Ok(()),
    }
}

#[test]
fn build_all_puzzles() -> Result<()> {
    let lib = load_puzzle_library();
    for puzzle in lib.puzzles() {
        time_it(
            format!("Building puzzle {} ({})", puzzle.display_name(), puzzle.id),
            || lib.build_puzzle(&puzzle.id).take_result_blocking(),
        )?;
    }
    Ok(())
}

fn load_puzzle_library() -> Library {
    let lib = Library::new();
    time_it("Loading all puzzles", || {
        lib.load_directory(Path::new("../lua"))
            .take_result_blocking()
    });
    lib
}

fn time_it<T>(task: impl fmt::Display, f: impl FnOnce() -> T) -> T {
    print!("{task} ...");
    std::io::stdout().flush().expect("error flushing stdout");
    let t1 = std::time::Instant::now();
    let ret = f();
    println!(" done in {:?}", t1.elapsed());
    ret
}
