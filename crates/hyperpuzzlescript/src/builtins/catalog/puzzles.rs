use hyperpuzzle_core::CatalogBuilder;

use crate::{Builtins, ErrorExt, EvalRequestTx, Result};

/// Adds the built-in functions.
pub fn define_in(
    builtins: &mut Builtins<'_>,
    catalog: &CatalogBuilder,
    eval_tx: &EvalRequestTx,
) -> Result<()> {
    let cat = catalog.clone();
    let tx = eval_tx.clone();
    builtins.set_fns(hps_fns![
        /// Adds a puzzle or puzzle generator to the catalog.
        ///
        /// This function always takes the following named arguments:
        ///
        /// - `id: Str` — ID for the puzzle or puzzle generator
        /// - `name: Str?` — Name of the puzzle or puzzle generator
        /// - `aliases: List[Str]?` — List of aliases for the puzzle or puzzle
        ///   generator
        /// - `version: Str?` — Version of the puzzle or puzzle generator
        /// - `tags: Map?` — Tags for the puzzle or puzzle generator
        ///   `"ndeuclid"`)
        /// - `colors: Str?` — ID of the color system
        /// - `twists: Str?` — ID of the twist system
        /// - `engine: Str` — Name of the puzzle engine to use (e.g.,
        ///   `"ndeuclid"`)
        ///
        /// Generated puzzles inherit tags from their generator, so tags do not
        /// need to be specified twice. Generally, generators should only have
        /// tags that are shared by all puzzles they are capable of generating.
        ///
        /// ## Single puzzle
        ///
        /// When used to define a single puzzle, this function takes other
        /// keyword arguments depending on the value of `engine`.
        ///
        /// ## Puzzle generator
        ///
        /// When used to define a puzzle generator, this function takes the
        /// following additional named arguments:
        ///
        /// - `params: List[Map]`
        /// - `gen: Fn(..) -> Map`
        ///
        /// The map returned by `gen` must have certain keys depending on the
        /// value of `engine`, in addition to the following:
        ///
        /// - `name: Str` — Name for the puzzle (e.g., "3^3")
        /// - `aliases: List[Str]?` — List of aliases for the puzzle
        /// - `colors: Str?` — ID of the color system (if not specified for the
        ///   generator)
        /// - `twists: Str?` — ID of the twist system (if not specified for the
        ///   generator)
        #[kwargs(kwargs)]
        fn add_puzzle(ctx: EvalCtx) -> () {
            pop_kwarg!(kwargs, (engine, engine_span): String);
            let engine = ctx.runtime.puzzle_engine_callback(&engine, engine_span)?;
            let hps_gen = super::generators::hps_generator_from_kwargs(kwargs)?;
            engine
                .add_catalog_entries(&cat, &tx, ctx, hps_gen)
                .map_err(|e| e.to_full_diagnostic(ctx.caller_span))?;
        }
    ])?;

    let cat = catalog.clone();
    builtins.set_fns(hps_fns![
        /// Adds an existing puzzle to the puzzle list.
        fn add_puzzle_list_entry((id, id_span): String) -> () {
            cat.add_to_puzzle_list(&id.parse().at(id_span)?);
        }
    ])?;

    Ok(())
}
