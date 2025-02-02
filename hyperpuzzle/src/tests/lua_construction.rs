use std::io::Write;

use eyre::Result;
use hyperpuzzle_core::PuzzleLintOutput;

use super::{load_new_catalog, time_it};

/// Whether to lint experimental puzzles. (Non-experimental puzzles are always
/// linted.)
const LINT_EXPERIMENTAL: bool = false;

#[test]
fn lint_all_puzzle_definitions() -> Result<(), String> {
    let catalog = load_new_catalog();

    let mut fail_count = 0;

    let mut out = String::new();

    for puzzle in catalog.puzzles().objects() {
        if !LINT_EXPERIMENTAL && puzzle.meta.tags.is_experimental() {
            continue;
        }

        let puzzle_lint_output = time_it(format!("Linting puzzle {}", puzzle.meta.id), || {
            PuzzleLintOutput::from_spec(&puzzle)
        })
        .0;

        if !puzzle_lint_output.all_good() {
            fail_count += 1;

            out += &format!(
                "Puzzle {} has lint errors:\n",
                puzzle_lint_output.puzzle.meta.id,
            );

            let PuzzleLintOutput {
                puzzle: _,
                schema,
                missing_tags,
            } = puzzle_lint_output;

            if !missing_tags.is_empty() {
                out += &format!("  Schema {schema}");
                out += "  Missing tags:\n";
                for tag in missing_tags {
                    out += &format!("    {tag:?}\n")
                }
            }
        }
    }

    // TODO: test output of puzzle generators

    if fail_count == 0 {
        Ok(())
    } else {
        std::fs::File::create("../lint_output.txt")
            .unwrap()
            .write(out.as_bytes())
            .unwrap();

        Err(format!("{fail_count} puzzles have lint errors"))
    }
}

#[test]
fn build_all_puzzles() -> Result<(), String> {
    let catalog = load_new_catalog();
    let mut failed = vec![];
    let mut times = vec![];
    let t1 = std::time::Instant::now();
    let puzzle_catalog = catalog.puzzles();
    for puzzle in puzzle_catalog.objects() {
        if puzzle.meta.tags.get("big").is_some_and(|v| v.is_present()) {
            println!(
                "Skipping big puzzle {} ({})",
                puzzle.meta.name, puzzle.meta.id,
            );
            continue;
        }

        let (result, time) = time_it(
            format!("Building puzzle {} ({})", puzzle.meta.name, puzzle.meta.id),
            || catalog.build_puzzle_blocking(&puzzle.meta.id),
        );
        match result {
            Ok(_) => {
                times.push((time, puzzle.meta.name.clone()));
            }
            Err(_) => {
                println!("Error building {}!", puzzle.meta.name);
                failed.push(puzzle);
            }
        }
    }
    let total_build_time = t1.elapsed();

    times.sort();
    println!();
    println!("Sorted:");
    for (time, puzzle) in times {
        println!("  {time:<11?} {puzzle}");
    }

    println!();
    println!("Built all puzzles in {total_build_time:?}");

    if failed.is_empty() {
        Ok(())
    } else {
        let fail_count = failed.len();
        println!();
        println!("{fail_count} puzzles failed to build:");
        for puzzle in failed {
            println!("  {} ({})", puzzle.meta.name, puzzle.meta.id);
        }
        Err(format!("{fail_count} puzzles failed to build:"))
    }
}

#[test]
fn build_7x7x7x7() {
    let lib = load_new_catalog();
    let (result, time) = time_it("Building puzzle 7x7x7x7", || {
        lib.build_puzzle_blocking("ft_hypercube:7")
    });
    result.expect("failed to build puzzle");
    println!("Done in {time:?}");
}
