use std::sync::Arc;

use ecow::eco_format;
use eyre::eyre;
use hyperpuzzle_core::Catalog;
use hyperpuzzle_core::catalog::{BuildTask, Generator, TwistSystemSpec};
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
        /// Adds a twist system to the catalog.
        ///
        /// This function takes the following named arguments:
        ///
        /// - `id: Str`
        /// - `name: Str?`
        /// - `engine: Str`
        ///
        /// The function takes other keyword arguments depending on the value of
        /// `engine`.
        #[kwargs(kwargs)]
        fn add_twist_system(ctx: EvalCtx) -> () {
            cat.add_twist_system(Arc::new(twist_system_from_kwargs(ctx, kwargs, &cat, &tx)?))
                .at(ctx.caller_span)?
        }
    ])?;

    let cat = catalog.clone();
    let tx = eval_tx.clone();
    builtins.set_fns(hps_fns![
        /// Adds a twist system generator to the catalog.
        ///
        /// This function takes the following named arguments:
        ///
        /// - `id: Str`
        /// - `name: Str?`
        /// - `version: Str?`
        /// - `params: List[Map]`
        /// - `gen: Fn(..) -> Map`
        #[kwargs(
            id: String,
            name: String = {
                ctx.warn(eco_format!("missing `name` for twist system generator `{id}`"));
                id.clone()
            },
            params: Vec<Spanned<Arc<Map>>> ,
            (r#gen, gen_span): Arc<FnValue>,
        )]
        fn add_twist_system_generator(ctx: EvalCtx) -> () {
            let caller_span = ctx.caller_span;

            let cat2 = cat.clone();
            let tx = tx.clone();

            let meta = super::generators::GeneratorMeta {
                id,
                params: super::generators::params_from_array(params)?,
                gen_fn: r#gen,
                gen_span,
            };

            let spec = Generator {
                id: meta.id.clone(),
                name,
                params: meta.params.clone(),
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
                                twist_system_from_kwargs(&mut ctx, spec, &cat2, &tx2).map(Arc::new)
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

            cat.add_twist_system_generator(Arc::new(spec))
                .at(ctx.caller_span)?
        }
    ])
}

fn twist_system_from_kwargs(
    ctx: &mut EvalCtx<'_>,
    mut kwargs: Map,
    catalog: &Catalog,
    eval_tx: &EvalRequestTx,
) -> Result<TwistSystemSpec> {
    pop_kwarg!(kwargs, id: String);
    pop_kwarg!(kwargs, name: String = {
        ctx.warn(eco_format!("missing `name` for twist system `{id}`"));
        id.clone()
    });
    pop_kwarg!(kwargs, (engine, engine_span): Str);

    let meta = crate::IdAndName { id, name };

    let Some(engine) = ctx
        .runtime
        .twist_system_engines
        .get(&*engine)
        .map(Arc::clone)
    else {
        let engine_list = ctx.runtime.twist_system_engines.keys().collect_vec();
        return Err(
            format!("unknown engine {engine:?}; supported engines: {engine_list:?}",)
                .at(engine_span),
        );
    };

    engine.new(ctx, meta, kwargs, catalog.clone(), eval_tx.clone())
}
