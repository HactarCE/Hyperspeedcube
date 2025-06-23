use std::sync::Arc;

use ecow::eco_format;
use hyperpuzzle_core::{Catalog, PuzzleListMetadata, TagSet};
use itertools::Itertools;

use crate::{ErrorExt, EvalRequestTx, Map, Result, Scope, Str};

/// Adds the built-in functions to the scope.
pub fn define_in(scope: &Scope, catalog: &Catalog, eval_tx: &EvalRequestTx) -> Result<()> {
    let cat = catalog.clone();
    let tx = eval_tx.clone();
    scope.register_builtin_functions(hps_fns![
        /// Adds a puzzle to the catalog.
        ///
        /// This function takes the following named arguments:
        ///
        /// - `id: Str`
        /// - `name: Str?`
        /// - `aliases: List[Str]?`
        /// - `tags: Map?`
        /// - `engine: Str`
        ///
        /// The function takes other keyword arguments depending on the value of
        /// `engine`.
        #[kwargs(kwargs)]
        fn add_puzzle(ctx: EvalCtx) -> () {
            pop_kwarg!(kwargs, id: String);
            pop_kwarg!(kwargs, name: String = {
                ctx.warn(eco_format!("missing `name` for puzzle `{id}`"));
                id.clone()
            });
            pop_kwarg!(kwargs, aliases: Vec<String> = vec![]);
            pop_kwarg!(kwargs, version: Option<String>);
            pop_kwarg!(kwargs, tags: Option<Arc<Map>>); // TODO
            pop_kwarg!(kwargs, (engine, engine_span): Str);

            let version = super::parse_version(ctx, &format!("puzzle `{id}`"), version.as_deref())?;

            let tags = TagSet::new(); // TODO

            let meta = PuzzleListMetadata {
                id,
                version,
                name,
                aliases,
                tags,
            };

            let Some(engine) = ctx.runtime.puzzle_engines.get(&*engine).map(Arc::clone) else {
                let engine_list = ctx.runtime.puzzle_engines.keys().collect_vec();
                return Err(format!(
                    "unknown engine {engine:?}; supported engines: {engine_list:?}",
                )
                .at(engine_span));
            };

            let spec = engine.new(ctx, meta, kwargs, cat.clone(), tx.clone())?;
            cat.add_puzzle(Arc::new(spec)).at(ctx.caller_span)?
        }
    ])
}
