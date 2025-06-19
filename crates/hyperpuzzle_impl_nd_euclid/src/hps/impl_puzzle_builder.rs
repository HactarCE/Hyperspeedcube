use std::sync::Arc;

use eyre::{Context, eyre};
use hyperpuzzle_core::{catalog::BuildTask, prelude::*};
use hyperpuzzlescript::*;

use crate::builder::*;

use super::{ArcMut, HpsNdEuclid};

impl hyperpuzzlescript::EngineCallback<PuzzleListMetadata, PuzzleSpec> for HpsNdEuclid {
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
                .insert_named("colors/system", TagValue::Str(color_system_id.into()))
                .map_err(|e| Error::User(e.to_string().into()).at(caller_span))?;
        }

        if let Some(twist_system_id) = twists.clone() {
            meta.tags
                .insert_named("twists/system", TagValue::Str(twist_system_id.into()))
                .map_err(|e| Error::User(e.to_string().into()).at(caller_span))?;
        }

        let meta = Arc::new(meta);

        Ok(PuzzleSpec {
            meta: Arc::clone(&meta),
            build: Box::new(move |build_ctx| {
                let builder = ArcMut::new(PuzzleBuilder::new(Arc::clone(&meta), ndim)?);

                // Build color system.
                if let Some(color_system_id) = &colors {
                    build_ctx.progress.lock().task = BuildTask::BuildingColors;
                    let colors = catalog
                        .build_blocking(color_system_id)
                        .map_err(|e| eyre!(e))?;
                    builder.shape().lock().colors = ColorSystemBuilder::unbuild(&colors)?;
                }

                // Build twist system.
                if let Some(twist_system_id) = &twists {
                    build_ctx.progress.lock().task = BuildTask::BuildingTwists;
                    let twists = catalog
                        .build_blocking(twist_system_id)
                        .map_err(|e| eyre!(e))?;
                    *builder.twists().lock() = TwistSystemBuilder::unbuild(&twists)?;
                }

                build_ctx.progress.lock().task = BuildTask::BuildingPuzzle;

                if let Some(remove_internals) = remove_internals {
                    builder.shape().lock().remove_internals = remove_internals;
                }
                if let Some(full_scramble_length) = scramble {
                    builder.lock().full_scramble_length = full_scramble_length
                        .try_into()
                        .wrap_err("bad scramble length")?;
                };

                let mut scope = Scope::default();
                scope.special.ndim = Some(ndim);
                let scope = Arc::new(scope);

                let build_fn = Arc::clone(&build);

                eval_tx.eval_blocking(move |runtime| {
                    let mut ctx = EvalCtx {
                        scope: &scope,
                        runtime,
                        caller_span,
                        exports: &mut None,
                    };
                    build_fn
                        .call_at(
                            build_span,
                            build_span,
                            &mut ctx,
                            vec![builder.clone().at(caller_span)],
                            Map::new(),
                        )
                        .map_err(|e| {
                            let s = e.to_string(&*ctx.runtime);
                            ctx.runtime.report_diagnostic(e);
                            eyre!(s)
                        })?;

                    let b = builder.lock();

                    // Assign default piece type to remaining pieces.
                    b.shape.lock().mark_untyped_pieces()?;

                    b.build(Some(&build_ctx), &mut ctx.warnf())
                        .map(|ok| Redirectable::Direct(ok))
                })
            }),
        })
    }
}

impl CustomValue for ArcMut<PuzzleBuilder> {
    fn type_name(&self) -> &'static str {
        "euclid.PuzzleBuilder"
    }

    fn clone_dyn(&self) -> BoxDynValue {
        self.clone().into()
    }

    fn fmt(&self, f: &mut std::fmt::Formatter<'_>, _is_repr: bool) -> std::fmt::Result {
        let p = self.lock();
        write!(
            f,
            "{}(id = {:?}, name = {:?}, ndim = {:?})",
            self.type_name(),
            p.meta.id,
            p.meta.name,
            p.ndim()
        )
    }
}
