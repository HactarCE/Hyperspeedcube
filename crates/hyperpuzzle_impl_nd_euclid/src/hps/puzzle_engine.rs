use std::sync::Arc;

use eyre::Context;
use hyperpuzzle_core::ComponentList;
use hyperpuzzle_core::catalog::GeneratorOutput;
use hyperpuzzle_core::prelude::*;
use hyperpuzzle_core::util::MaybeAdHoc;
use hyperpuzzlescript::*;
use parking_lot::Mutex;

use super::HpsNdEuclid;
use crate::builder::*;
use crate::hps::{HpsAxisSystem, HpsPuzzle, HpsShape, HpsTwistSystem};

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
    ) -> Result<GeneratorOutput<Puzzle>> {
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

        meta.tags.set_opt_color_system(colors.as_deref());
        meta.tags.set_opt_twist_system(twists.as_deref());

        if let Err(e) = meta.tags.insert_named("ndim", TagValue::Int(ndim as i64)) {
            ctx.warn(e.to_string());
        }

        let meta = Arc::new(meta);

        Ok(GeneratorOutput {
            meta: Arc::clone(&meta),
            components: ComponentList::new(),
            build: Arc::new(move |build_ctx| {
                let logger = &build_ctx.catalog.logger;
                let builder = Arc::new(Mutex::new(PuzzleBuilder::new(Arc::clone(&meta), ndim)?));
                let id = &meta.id;

                // Build color system.
                if let Some(colors_id) = &colors {
                    builder.lock().shape.lock().colors = ColorSystemBuilder(MaybeAdHoc::Fixed(
                        build_ctx.build_str_blocking(colors_id)?,
                    ));
                } else {
                    logger.warn(format!("using ad-hoc color system for puzzle {id:?}"));
                }

                // Build twist system.
                if let Some(twists_id) = &twists {
                    builder.lock().twists = TwistSystemBuilder(MaybeAdHoc::Fixed(
                        build_ctx.build_str_blocking(twists_id)?,
                    ));
                } else {
                    logger.warn(format!("using ad-hoc color system for puzzle {id:?}"));
                }

                build_ctx.set_building::<Puzzle>();

                if let Some(remove_internals) = remove_internals {
                    builder.lock().shape.lock().remove_internals = remove_internals;
                }
                if let Some(full_scramble_length) = scramble {
                    builder.lock().full_scramble_length = full_scramble_length;
                };

                let mut scope = Scope::default();
                scope.special.ndim = Some(ndim);
                scope.special.puz =
                    Arc::new(Mutex::new(HpsPuzzle(builder.clone()).at(BUILTIN_SPAN)));
                scope.special.shape = Arc::new(Mutex::new(
                    HpsShape(builder.lock().shape.clone()).at(BUILTIN_SPAN),
                ));
                scope.special.twists = Arc::new(Mutex::new(
                    HpsTwistSystem(builder.lock().twists.clone()).at(BUILTIN_SPAN),
                ));
                scope.special.axes = Arc::new(Mutex::new(
                    HpsAxisSystem(builder.lock().twists.clone()).at(BUILTIN_SPAN),
                ));
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
                        .map_err(|e| ctx.runtime.report_and_convert_to_eyre(e))
                        .wrap_err("error building puzzle")?;

                    let b = builder.lock();

                    // Assign default piece type to remaining pieces.
                    b.shape.lock().mark_untyped_pieces()?;

                    b.build(Some(&build_ctx), &mut ctx.warnf())
                })
            }),
        })
    }
}
