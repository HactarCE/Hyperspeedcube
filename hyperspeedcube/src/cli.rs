use std::io::{Read, Write};

use eyre::{eyre, Context, Result};
use itertools::Itertools;
use serde::{Deserialize, Serialize};

// TODO: specify log file via CLI

/// Hyperspeedcube command-line interface
///
/// If no subcommand is specified, then the GUI is opened.
#[derive(Debug, clap::Parser)]
#[command(version, args_conflicts_with_subcommands = true)]
pub(crate) struct Args {
    #[command(subcommand)]
    pub subcommand: Option<Subcommand>,
    // /// Log file to open in the GUI.
    // #[arg(value_parser)]
    // pub input_file: Option<clio::Input>,
}

#[derive(clap::Subcommand, Debug)]
pub(crate) enum Subcommand {
    /// Outputs information about a puzzle.
    Puzzle {
        /// Puzzle ID, such as `ft_cube:3`
        puzzle_id: String,
    },
    /// Verifies a log file and outputs JSON.
    Verify {
        /// Log file to verify, use '-' for stdin.
        #[arg(value_parser)]
        log_file: clio::Input,

        /// Don't verify that the puzzle was actually solved.
        #[arg(long)]
        skip_simulation: bool,
    },
}

pub(crate) fn exec(subcommand: Subcommand) -> Result<()> {
    match subcommand {
        Subcommand::Puzzle { puzzle_id } => {
            hyperpuzzle::load_global_catalog();
            let puzzle = hyperpuzzle::catalog()
                .build_puzzle_spec_blocking(&puzzle_id)
                .map_err(|e| eyre!("error building puzzle: {e}"))?;
            write_json_output(&puzzle.meta)
        }

        Subcommand::Verify {
            mut log_file,
            skip_simulation,
        } => {
            hyperpuzzle::load_global_catalog();
            let mut buffer = String::new();
            log_file
                .read_to_string(&mut buffer)
                .context("error reading log file")?;
            let (log_file, _warnings) = hyperpuzzle_log::LogFile::deserialize(&buffer)
                .context("error deserializing log file")?;

            hyperpuzzle::load_global_catalog();
            let catalog = hyperpuzzle::catalog();

            let facts = log_file
                .solves
                .iter()
                .filter_map(|solve| {
                    if !skip_simulation {
                        hyperpuzzle_log::verify::verify(&catalog, solve)
                    } else {
                        hyperpuzzle_log::verify::verify_without_checking_solution(&catalog, solve)
                    }
                })
                .collect_vec();

            write_json_output(&facts)
        }
    }
}

fn write_json_output<T: Serialize>(value: &T) -> Result<()> {
    serde_json::to_writer_pretty(std::io::stdout(), value)
        .context("error writing verification to output")
}
