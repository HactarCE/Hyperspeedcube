use hyperpuzzle_core::CatalogBuilder;

use crate::{Builtins, EvalRequestTx, Result};

/// Adds the built-in functions.
pub fn define_in(
    builtins: &mut Builtins<'_>,
    catalog: &CatalogBuilder,
    eval_tx: &EvalRequestTx,
) -> Result<()> {
    let cat = catalog.clone();
    let tx = eval_tx.clone();
    builtins.set_fns(hps_fns![
        /// Adds a twist system or twist system generator to the catalog.
        ///
        /// ## Single twist system
        ///
        /// When used to define a single twist system, this function takes the
        /// following named arguments:
        ///
        /// - `id: Str` — ID for the twist system (e.g., `"cubic_ft"`)
        /// - `name: Str?` — Name for the twist system (e.g., `"FT Cubic"`)
        /// - `engine: Str` — Name of the twist system engine to use (e.g.,
        ///   `"ndeuclid"`)
        ///
        /// The function takes other keyword arguments depending on the value of
        /// `engine`.
        ///
        /// ## Twist system generator
        ///
        /// When used to define a twist system generator, this function takes
        /// the following named arguments:
        ///
        /// - `id: Str` — ID for the twist system generator (e.g., `"ngon_ft"`)
        /// - `engine: Str` — Name of the twist system engine to use (e.g.,
        ///   "ndeuclid")
        /// - `params: List[Map]` — List of generator parameters
        /// - `gen: Fn(..) -> Map` — Generator function
        ///
        /// The map returned by `gen` must have certain keys depending on the
        /// value of `engine`, in addition to the following:
        ///
        /// - `name: Str` — Name for the twist system (e.g., "FT {5}")
        #[kwargs(kwargs)]
        fn add_twist_system(ctx: EvalCtx) -> () {
            pop_kwarg!(kwargs, (engine, engine_span): String);
            let engine = ctx
                .runtime
                .twist_system_engine_callback(&engine, engine_span)?;
            let hps_gen = super::generators::hps_generator_from_kwargs(ctx, kwargs)?;
            engine
                .add_catalog_entries(&cat, &tx, ctx, hps_gen)
                .map_err(|e| e.to_full_diagnostic(ctx.caller_span))?;
        }
    ])
}
