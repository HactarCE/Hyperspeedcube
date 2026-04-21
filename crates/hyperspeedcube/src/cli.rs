use std::sync::Arc;
use std::{io::Read, str::FromStr};

use eyre::{Context, Result, eyre};
use hyperpuzzle::{CatalogId, Puzzle};
use itertools::Itertools;
use serde::Serialize;

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
    /// Print program info and credits in Markdown.
    About,
    /// Print information about a puzzle or generator as JSON.
    Puzzle {
        /// Puzzle or generator ID(s) (such as `ft_cube` or `ft_cube(3)`)
        ids: Vec<String>,
    },
    /// Print all non-experimental puzzle and puzzle generator IDs.
    Puzzles {
        /// List only non-generated puzzles.
        #[arg(short, long)]
        puzzles: bool,
        /// List only generators.
        #[arg(short, long)]
        generators: bool,
        /// List only generator examples.
        #[arg(short, long)]
        examples: bool,

        /// Include experimental puzzles.
        #[arg(short = 'x', long)]
        experimental: bool,

        /// Query expression(s) to search for.
        query: Vec<String>,
    },
    /// Print all tags.
    Tags,
    /// Verify a log file and print info about it as JSON.
    Verify {
        /// Log file to verify, use '-' for stdin.
        #[arg(value_parser)]
        log_file: clio::Input,

        /// Don't verify that the puzzle was actually solved.
        #[arg(long)]
        fast: bool,
    },
}

pub(crate) fn exec(subcommand: Subcommand) -> Result<()> {
    match subcommand {
        Subcommand::About => {
            hyperpuzzle::load_global_catalog();
            println!("{}", crate::about_text());
            Ok(())
        }

        Subcommand::Puzzle { ids } => {
            hyperpuzzle::load_global_catalog();
            let catalog = hyperpuzzle::catalog();
            let mut requested_puzzles = vec![];
            for puzzle_id in ids {
                let catalog_id = CatalogId::from_str(&puzzle_id)
                    .map_err(|e| eyre!("error parsing ID string: {e}"))?;
                let puzzle_meta = catalog
                    .get_puzzle_metadata_blocking(&catalog_id)
                    .wrap_err("error building puzzle")?;
                requested_puzzles.push(puzzle_meta.to_cli());
            }
            write_json_output(&requested_puzzles)?;
            Ok(())
        }

        Subcommand::Puzzles {
            puzzles,
            generators,
            examples,

            experimental,

            query,
        } => {
            let all = (!puzzles && !generators && !examples) || (puzzles && generators && examples);
            hyperpuzzle::load_global_catalog();
            let catalog = hyperpuzzle::catalog();

            let mut entries = catalog.puzzle_list.clone();

            // Filter by type
            if !all {
                entries.retain(|meta| {
                    let Some(generator) = catalog.puzzles.generators.get(&*meta.id.base) else {
                        log::warn!(
                            "puzzle list entry {} has no corresponding generator",
                            meta.id
                        );
                        return false;
                    };
                    let is_generator = !generator.params.is_empty();
                    let is_example = meta.tags.has_present("generated");

                    generators && is_generator
                        || examples && is_example
                        || puzzles && !is_generator && !is_example
                });
            }

            // Filter by experimental
            if !experimental {
                entries.retain(|meta| !meta.tags.is_experimental());
            }

            // Filter by query
            let query_str = query.join(" ");
            let ids = if !query_str.is_empty() {
                let query = crate::gui::Query::from_str(&query_str);
                entries
                    .iter()
                    .filter_map(|entry| query.try_match(entry))
                    .sorted_unstable()
                    .map(|query_match| &query_match.object.id)
                    .collect_vec()
            } else {
                entries
                    .iter()
                    .sorted_unstable()
                    .map(|meta| &meta.id)
                    .collect_vec()
            };

            for id in ids {
                println!("{id}");
            }

            Ok(())
        }

        Subcommand::Tags => {
            for tag in hyperpuzzle::TAGS.all_tags() {
                println!("{tag}");
            }
            Ok(())
        }

        Subcommand::Verify { mut log_file, fast } => {
            log::trace!("Loading global catalog ...");
            hyperpuzzle::load_global_catalog();

            let mut buffer = String::new();
            log_file
                .read_to_string(&mut buffer)
                .context("error reading log file")?;
            log::trace!("Deserializing log file ...");
            let (log_file, _warnings) =
                hyperpuzzle_log::deserialize(&buffer).context("error deserializing log file")?;

            let catalog = hyperpuzzle::catalog();

            let facts = log_file
                .solves
                .iter()
                .filter_map(|solve| {
                    hyperpuzzle_log::verify::verify(
                        &catalog,
                        solve,
                        if fast {
                            hyperpuzzle_log::verify::VerificationOptions::QUICK
                        } else {
                            hyperpuzzle_log::verify::VerificationOptions::FULL
                        },
                    )
                    .map_err(|e| eprintln!("{e}"))
                    .ok()
                })
                .collect_vec();

            write_json_output(&facts)
        }
    }
}

fn write_json_output<T: Serialize>(value: &T) -> Result<()> {
    serde_json::to_writer_pretty(std::io::stdout(), value)
        .context("error serializing data and writing to stdout")?;
    println!();
    Ok(())
}
