use std::sync::Arc;

use eyre::Context;
use hyperpuzzle_core::ComponentList;
use hyperpuzzle_core::catalog::GeneratorOutput;
use hyperpuzzle_core::prelude::*;
use hyperpuzzle_core::util::MaybeAdHoc;
use hyperpuzzlescript::builtins::catalog::HpsExports;
use hyperpuzzlescript::*;
use parking_lot::Mutex;

use super::HpsNdEuclid;
use crate::builder::*;
use crate::hps::{HpsAxisSystem, HpsTwistSystem};

impl hyperpuzzlescript::EngineCallback<TwistSystem> for HpsNdEuclid {
    fn name(&self) -> String {
        self.to_string()
    }

    fn new(
        &self,
        ctx: &mut EvalCtx<'_>,
        meta: CatalogMetadata,
        kwargs: Map,
        eval_tx: EvalRequestTx,
    ) -> Result<GeneratorOutput<TwistSystem>> {
        let caller_span = ctx.caller_span;

        unpack_kwargs!(kwargs, ndim: u8, (build, build_span): Arc<FnValue>);

        let meta = Arc::new(meta);

        Ok(GeneratorOutput {
            meta: Arc::clone(&meta),
            components: ComponentList::new(),
            build: Arc::new(move |build_ctx| {
                let id = meta.id.clone();
                let builder = Arc::new(Mutex::new(AdHocTwistSystemBuilder::new(
                    id.clone(),
                    Some(meta.name.clone()),
                    ndim,
                )));

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
                    let exports = build_fn
                        .call(build_span, &mut ctx, vec![], Map::new())
                        .map_err(|e| ctx.runtime.report_and_convert_to_eyre(e))
                        .wrap_err("error building twist system")?;

                    let mut b = builder.lock();
                    if let Ok(exports_map) = exports.to::<Arc<Map>>() {
                        b.hps_exports = HpsExports((*exports_map).clone());
                    }

                    b.build(Some(&build_ctx), &mut ctx.warnf())
                })
            }),
        })
    }
}
