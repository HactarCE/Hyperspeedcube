use std::sync::Arc;

use hyperpuzzle_core::prelude::*;
use hyperpuzzle_core::util::MaybeAdHoc;
use hyperpuzzlescript::builtins::catalog::HpsExports;
use hyperpuzzlescript::engine::HpsEngineError;
use hyperpuzzlescript::*;
use parking_lot::Mutex;

use crate::builder::*;
use crate::hps::{HpsAxisSystem, HpsTwistSystem};

pub struct NdEuclidTwistSystemEngine;

impl HpsEngine for NdEuclidTwistSystemEngine {
    fn add_catalog_entries(
        &self,
        catalog: &CatalogBuilder,
        eval_tx: &EvalRequestTx,
        ctx: &mut EvalCtx<'_>,
        hps_gen: engine::HpsGenerator,
    ) -> Result<(), HpsEngineError> {
        let caller_span = ctx.caller_span;

        catalog.add::<TwistSystem>(hps_gen.make_generator(
            eval_tx,
            move |build_ctx, tx, kwargs| {
                let id = build_ctx.id();

                unpack_kwargs!(
                    kwargs,
                    ndim: u8,
                    (build, build_span): Arc<FnValue>,
                );

                let builder = Arc::new(Mutex::new(AdHocTwistSystemBuilder::new(id.clone(), ndim)));

                let mut scope = Scope::default();
                scope.special.ndim = Some(ndim);
                scope.special.twists = Arc::new(Mutex::new(
                    HpsTwistSystem(TwistSystemBuilder(MaybeAdHoc::AdHoc(builder.clone())))
                        .at(BUILTIN_SPAN),
                ));
                scope.special.axes = Arc::new(Mutex::new(
                    HpsAxisSystem(TwistSystemBuilder(MaybeAdHoc::AdHoc(builder.clone())))
                        .at(BUILTIN_SPAN),
                ));
                scope.special.id = Some(id.to_string().into());
                let exports = tx.eval_blocking(Arc::new(scope), move |ctx| {
                    build.call(build_span, ctx, vec![], Map::new())
                })?;

                let mut b = builder.lock();
                if let Ok(exports_map) = exports.to::<Arc<Map>>() {
                    b.hps_exports = HpsExports((*exports_map).clone());
                }

                Ok(b.build(&build_ctx)?)
            },
        ))?;

        Ok(())
    }
}
