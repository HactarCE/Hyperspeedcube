use std::collections::HashMap;
use std::sync::Arc;

use ecow::eco_format;
use eyre::eyre;
use hyperpuzzle_core::catalog::BuildTask;
use hyperpuzzle_core::{Catalog, PuzzleListMetadata, PuzzleSpec, PuzzleSpecGenerator, TagSet};
use itertools::Itertools;

use crate::{
    Builtins, ErrorExt, EvalCtx, EvalRequestTx, FnValue, Map, Result, Scope, Spanned, Str,
};

/// Adds the built-in functions.
pub fn define_in(
    builtins: &mut Builtins<'_>,
    catalog: &Catalog,
    eval_tx: &EvalRequestTx,
) -> Result<()> {
    let cat = catalog.clone();
    let tx = eval_tx.clone();
    builtins.set_fns(hps_fns![
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
            cat.add_puzzle(Arc::new(puzzle_spec_from_kwargs(ctx, kwargs, &cat, &tx)?))
                .at(ctx.caller_span)?
        }
    ])?;

    let cat = catalog.clone();
    let tx = eval_tx.clone();
    builtins.set_fns(hps_fns![
        /// Adds a puzzle generator to the catalog.
        ///
        /// This function takes the following named arguments:
        ///
        /// - `id: Str`
        /// - `name: Str?`
        /// - `aliases: List[Str]?`
        /// - `version: Str?`
        /// - `tags: Map?`
        /// - `params: List[Map]`
        /// - `examples: List[Map]`
        /// - `gen: Fn(..) -> Map`
        ///
        /// Other keyword arguments are copied into the output of `gen`.
        #[kwargs(kwargs)]
        fn add_puzzle_generator(ctx: EvalCtx) -> () {
            pop_kwarg!(kwargs, id: String);
            pop_kwarg!(kwargs, name: String = {
                ctx.warn(eco_format!("missing `name` for puzzle generator `{id}`"));
                id.clone()
            });
            pop_kwarg!(kwargs, aliases: Vec<String> = vec![]);
            pop_kwarg!(kwargs, version: Option<String>);
            pop_kwarg!(kwargs, tags: Option<Arc<Map>>);
            pop_kwarg!(kwargs, params: Vec<Spanned<Arc<Map>>> );
            pop_kwarg!(kwargs, examples: Vec<Spanned<Arc<Map>>> = vec![]);
            pop_kwarg!(kwargs, (r#gen, gen_span): Arc<FnValue>);

            let caller_span = ctx.caller_span;

            let cat2 = cat.clone();
            let tx = tx.clone();

            let version =
                super::parse_version(ctx, &format!("puzzle generator `{id}`"), version.as_deref())?;

            let tags = TagSet::new(); // TODO

            let meta = super::generators::GeneratorMeta {
                id,
                params: super::generators::params_from_array(params)?,
                gen_fn: r#gen,
                gen_span,
                extra: kwargs,
            };

            let spec = PuzzleSpecGenerator {
                meta: Arc::new(PuzzleListMetadata {
                    id: meta.id.clone(),
                    version,
                    name,
                    aliases,
                    tags,
                }),
                params: meta.params.clone(),
                examples: HashMap::new(), // TODO
                generate: Box::new(move |build_ctx, param_values| {
                    build_ctx.progress.lock().task = BuildTask::GeneratingSpec;

                    let cat2 = cat2.clone();
                    let tx2 = tx.clone();

                    let scope = Scope::new();
                    let meta = meta.clone();

                    tx.clone().eval_blocking(move |runtime| {
                        let mut ctx = EvalCtx {
                            scope: &scope,
                            runtime,
                            caller_span,
                            exports: &mut None,
                        };

                        // IIFE to mimic try_block
                        (|| {
                            meta.generate_spec(&mut ctx, param_values)?.try_map(|spec| {
                                // TODO: add tags
                                puzzle_spec_from_kwargs(&mut ctx, spec, &cat2, &tx2).map(Arc::new)
                            })
                        })()
                        .map_err(|e| {
                            let s = e.to_string(&*ctx.runtime);
                            ctx.runtime.report_diagnostic(e);
                            eyre!(s)
                        })
                    })
                }),
            };

            cat.add_puzzle_generator(Arc::new(spec))
                .at(ctx.caller_span)?
        }
    ])
}

fn puzzle_spec_from_kwargs(
    ctx: &mut EvalCtx<'_>,
    mut kwargs: Map,
    catalog: &Catalog,
    eval_tx: &EvalRequestTx,
) -> Result<PuzzleSpec> {
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
        return Err(
            format!("unknown engine {engine:?}; supported engines: {engine_list:?}",)
                .at(engine_span),
        );
    };

    engine.new(ctx, meta, kwargs, catalog.clone(), eval_tx.clone())
}
