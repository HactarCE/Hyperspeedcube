use std::sync::Arc;

use eyre::eyre;
use hyperpuzzle_core::catalog::BuildTask;
use hyperpuzzle_core::prelude::*;
use hyperpuzzlescript::*;

use super::{ArcMut, HpsNdEuclid};
use crate::builder::*;

impl hyperpuzzlescript::EngineCallback<PuzzleListMetadata, PuzzleSpec> for HpsNdEuclid {
    fn name(&self) -> String {
        self.to_string()
    }

    fn new(
        &self,
        ctx: &mut EvalCtx<'_>,
        mut meta: PuzzleListMetadata,
        kwargs: Map,
        catalog: Catalog,
        eval_tx: EvalRequestTx,
    ) -> Result<PuzzleSpec> {
        let caller_span = ctx.caller_span;

        unpack_kwargs!(
            kwargs,
            colors: Option<String>, // TODO: string or array of strings (gen ID + params)
            twists: Option<String>, // TODO: string or array of strings (gen ID + params)
            ndim: u8,
            (build, build_span): Arc<FnValue>,
            remove_internals: Option<bool>,
            scramble: Option<u32>,
        );

        if let Some(color_system_id) = colors.clone() {
            meta.tags
                .insert_named("colors/system", TagValue::Str(color_system_id))
                .map_err(|e| Error::User(e.to_string().into()).at(caller_span))?;
        }

        if let Some(twist_system_id) = twists.clone() {
            meta.tags
                .insert_named("twists/system", TagValue::Str(twist_system_id))
                .map_err(|e| Error::User(e.to_string().into()).at(caller_span))?;
        }

        if let Err(e) = meta.tags.insert_named("ndim", TagValue::Int(ndim as i64)) {
            ctx.warn(e.to_string());
        }

        let meta = Arc::new(meta);

        Ok(PuzzleSpec {
            meta: Arc::clone(&meta),
            build: Box::new(move |build_ctx| {
                let builder = ArcMut::new(
                    PuzzleBuilder::new(Arc::clone(&meta), ndim).map_err(|e| format!("{e:#}"))?,
                );
                let id = meta.id.clone();

                // Build color system.
                if let Some(color_system_id) = &colors {
                    build_ctx.progress.lock().task = BuildTask::BuildingColors;
                    let colors = catalog.build_blocking(color_system_id)?;
                    builder.shape().lock().colors =
                        ColorSystemBuilder::unbuild(&colors).map_err(|e| format!("{e:#}"))?;
                }

                // Build twist system.
                if let Some(twist_system_id) = &twists {
                    build_ctx.progress.lock().task = BuildTask::BuildingTwists;
                    let twists = catalog.build_blocking(twist_system_id)?;
                    *builder.twists().lock() =
                        TwistSystemBuilder::unbuild(&twists).map_err(|e| format!("{e:#}"))?;
                }

                build_ctx.progress.lock().task = BuildTask::BuildingPuzzle;

                if let Some(remove_internals) = remove_internals {
                    builder.shape().lock().remove_internals = remove_internals;
                }
                if let Some(full_scramble_length) = scramble {
                    builder.lock().full_scramble_length = full_scramble_length;
                };

                let mut scope = Scope::default();
                scope.special.ndim = Some(ndim);
                scope.special.puz = builder.clone().at(BUILTIN_SPAN);
                scope.special.shape = builder.shape().at(BUILTIN_SPAN);
                scope.special.twists = builder.twists().at(BUILTIN_SPAN);
                scope.special.axes = builder.axes().at(BUILTIN_SPAN);
                scope.special.id = Some((&id).into());
                let scope = Arc::new(scope);

                let build_fn = Arc::clone(&build);

                eval_tx
                    .eval_blocking(move |runtime| {
                        let mut ctx = EvalCtx {
                            scope: &scope,
                            runtime,
                            caller_span,
                            exports: &mut None,
                            stack_depth: 0,
                        };
                        build_fn
                            .call(build_span, &mut ctx, vec![], Map::new())
                            .map_err(|e| {
                                ctx.runtime.report_diagnostic(e);
                                eyre!("unable to build puzzle `{id}`; see HPS logs")
                            })?;

                        let b = builder.lock();

                        // Assign default piece type to remaining pieces.
                        b.shape.lock().mark_untyped_pieces()?;

                        b.build(Some(&build_ctx), &mut ctx.warnf())
                            .map(Redirectable::Direct)
                    })
                    .map_err(|e| format!("{e:#}"))
            }),
        })
    }
}
