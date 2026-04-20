use std::{str::FromStr, sync::Arc};

use eyre::{Context, eyre};
use hyperpuzzle_core::catalog::BuildTask;
use hyperpuzzle_core::prelude::*;
use hyperpuzzlescript::*;

use super::{ArcMut, HpsNdEuclid};
use crate::builder::*;

impl hyperpuzzlescript::EngineCallback<Puzzle> for HpsNdEuclid {
    fn name(&self) -> String {
        self.to_string()
    }

    fn new(
        &self,
        ctx: &mut EvalCtx<'_>,
        mut meta: CatalogMetadata,
        kwargs: Map,
        eval_tx: EvalRequestTx,
    ) -> Result<LazyCatalogConstructor<Puzzle>> {
        let caller_span = ctx.caller_span;

        unpack_kwargs!(
            kwargs,
            colors: Option<String>,
            twists: Option<String>,
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

        Ok(LazyCatalogConstructor {
            meta: Arc::clone(&meta),
            build: Box::new(move |build_ctx| {
                let builder = ArcMut::new(PuzzleBuilder::new(Arc::clone(&meta), ndim)?);
                let id = meta.id.clone();

                // Build color system.
                if let Some(color_system_id) = &colors {
                    build_ctx.progress.lock().task = BuildTask::BuildingColors;
                    let colors = build_ctx
                        .catalog
                        .build_blocking(
                            &CatalogId::from_str(color_system_id).map_err(|e| eyre!("{e}"))?,
                        )
                        .map_err(|e| clone_eyre(&e))
                        .wrap_err("error building color system")?;
                    builder.shape().lock().colors = ColorSystemBuilder::unbuild(&colors)?;
                }

                // Build twist system.
                if let Some(twist_system_id) = &twists {
                    build_ctx.progress.lock().task = BuildTask::BuildingTwists;
                    let twists = build_ctx
                        .catalog
                        .build_blocking(
                            &CatalogId::from_str(twist_system_id).map_err(|e| eyre!("{e}"))?,
                        )
                        .map_err(|e| clone_eyre(&e))
                        .wrap_err("error building twist system")?;
                    *builder.twists().lock() = TwistSystemBuilder::unbuild(&twists)?;
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
                scope.special.id = Some(id.to_string().into());
                let scope = Arc::new(scope);

                let build_fn = Arc::clone(&build);

                eval_tx.eval_blocking(move |runtime| {
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
                            ctx.runtime
                                .report_and_convert_to_eyre(e)
                                .wrap_err("error building puzzle")
                        })?;

                    let b = builder.lock();

                    // Assign default piece type to remaining pieces.
                    b.shape.lock().mark_untyped_pieces()?;

                    b.build(Some(&build_ctx), &mut ctx.warnf())
                })
            }),
        })
    }
}

#[track_caller]
fn clone_eyre(e: &eyre::Report) -> eyre::Report {
    if let Some(e) = e.downcast_ref::<FormattedFullDiagnostic>().cloned() {
        eyre!(e)
    } else {
        eyre!("{e}") // cursed reformatting of eyre::Report
    }
}
