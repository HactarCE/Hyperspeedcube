use std::sync::Arc;

use ecow::eco_format;
use hyperpuzzle_core::catalog::BuildTask;
use hyperpuzzle_core::{
    CatalogBuilder, CatalogId, CatalogMetadata, TwistSystem, TwistSystemGenerator,
};
use itertools::Itertools;

use crate::{
    Builtins, ErrorExt, EvalCtx, EvalRequestTx, FnValue, LazyCatalogConstructor, Map, Result,
    Spanned, Str,
};

/// Adds the built-in functions.
pub fn define_in(
    builtins: &mut Builtins<'_>,
    catalog: &CatalogBuilder,
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
            let lazy_twist_system = twist_system_from_kwargs(ctx, kwargs, &tx)?;
            cat.add_generator(Arc::new(lazy_twist_system.into_generator()))
                .at(ctx.caller_span)?;
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
        ///
        /// Other keyword arguments are copied into the output of `gen`.
        #[kwargs(kwargs)]
        fn add_twist_system_generator(ctx: EvalCtx) -> () {
            pop_kwarg!(kwargs, (id, id_span): String);
            pop_kwarg!(kwargs, name: String = {
                ctx.warn(eco_format!("missing `name` for twist system generator `{id}`"));
                id.clone()
            });
            pop_kwarg!(kwargs, (params, params_span): Vec<Spanned<Arc<Map>>>);
            pop_kwarg!(kwargs, (r#gen, gen_span): Arc<FnValue>);

            let tx = tx.clone();
            let hps_gen = super::generators::HpsGenerator {
                def_span: ctx.caller_span,
                id: CatalogId::new(id, []).ok_or("invalid ID").at(id_span)?,
                id_span,
                params: super::generators::params_from_array(params)?,
                params_span,
                gen_fn: r#gen,
                gen_span,
                extra: Arc::new(kwargs),
            };

            let tx2 = tx.clone();
            let hps_gen2 = hps_gen.clone();
            let generator = TwistSystemGenerator {
                meta: Arc::new(CatalogMetadata::simple(hps_gen.id.clone(), name.clone())),
                params: hps_gen.params.clone(),
                generate_meta: Box::new(move |build_ctx, param_values| {
                    build_ctx.progress.lock().task = BuildTask::BuildingTwists;
                    hps_gen.generate_on_hps_thread(&tx, param_values, |ctx, mut kwargs| {
                        pop_twist_system_meta_from_kwargs(ctx, &mut kwargs).map(Arc::new)
                    })
                }),
                generate: Box::new(move |build_ctx, param_values| {
                    build_ctx.progress.lock().task = BuildTask::BuildingTwists;
                    let tx3 = tx2.clone();
                    hps_gen2
                        .generate_on_hps_thread(&tx2, param_values, move |ctx, kwargs| {
                            twist_system_from_kwargs(ctx, kwargs, &tx3).map(Arc::new)
                        })?
                        .try_map(|lazy_twist_system| (lazy_twist_system.build)(build_ctx))
                }),
            };

            cat.add_generator(Arc::new(generator)).at(ctx.caller_span)?;
        }
    ])
}

fn pop_twist_system_meta_from_kwargs(
    ctx: &mut EvalCtx<'_>,
    kwargs: &mut Map,
) -> Result<CatalogMetadata> {
    pop_kwarg!(*kwargs, (id, id_span): String);
    pop_kwarg!(*kwargs, name: String = {
        ctx.warn(eco_format!("missing `name` for twist system `{id}`"));
        id.clone()
    });
    Ok(CatalogMetadata::simple(id.parse().at(id_span)?, name))
}

fn twist_system_from_kwargs(
    ctx: &mut EvalCtx<'_>,
    mut kwargs: Map,
    eval_tx: &EvalRequestTx,
) -> Result<LazyCatalogConstructor<TwistSystem>> {
    let meta = pop_twist_system_meta_from_kwargs(ctx, &mut kwargs)?;
    pop_kwarg!(kwargs, (engine, engine_span): Str);

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

    engine.new(ctx, meta, kwargs, eval_tx.clone())
}
