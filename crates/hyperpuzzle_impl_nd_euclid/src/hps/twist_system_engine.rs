use std::sync::Arc;

use eyre::eyre;
use hyperpuzzle_core::catalog::TwistSystemSpec;
use hyperpuzzle_core::prelude::*;
use hyperpuzzlescript::*;

use super::{ArcMut, HpsNdEuclid};
use crate::builder::*;

impl hyperpuzzlescript::EngineCallback<IdAndName, TwistSystemSpec> for HpsNdEuclid {
    fn name(&self) -> String {
        self.to_string()
    }

    fn new(
        &self,
        ctx: &mut EvalCtx<'_>,
        meta: IdAndName,
        kwargs: Map,
        _catalog: Catalog,
        eval_tx: EvalRequestTx,
    ) -> Result<TwistSystemSpec> {
        let caller_span = ctx.caller_span;

        unpack_kwargs!(kwargs, ndim: u8, (build, build_span): Arc<FnValue>);

        Ok(TwistSystemSpec {
            id: meta.id.clone(),
            name: meta.name.clone(),
            build: Box::new(move |build_ctx| {
                let builder = ArcMut::new(TwistSystemBuilder::new_shared(
                    meta.id.clone(),
                    Some(meta.name.clone()),
                    ndim,
                ));

                let mut scope = Scope::default();
                scope.special.ndim = Some(ndim);
                scope.special.twists = builder.clone().at(BUILTIN_SPAN);
                scope.special.axes = builder.axes().at(BUILTIN_SPAN);
                let scope = Arc::new(scope);

                let build_fn = Arc::clone(&build);

                eval_tx.eval_blocking(move |runtime| {
                    let mut ctx = EvalCtx {
                        scope: &scope,
                        runtime,
                        caller_span,
                        exports: &mut None,
                    };
                    let exports = build_fn
                        .call(build_span, &mut ctx, vec![], Map::new())
                        .map_err(|e| {
                            let s = e.to_string(&*ctx.runtime);
                            ctx.runtime.report_diagnostic(e);
                            eyre!(s)
                        })?;

                    let mut b = builder.lock();
                    if let Ok(exports_map) = exports.to::<Arc<Map>>() {
                        b.hps_exports = exports_map;
                    }

                    let puzzle_id = None;
                    b.build(Some(&build_ctx), puzzle_id, &mut ctx.warnf())
                        .map(|ok| Redirectable::Direct(Arc::new(ok)))
                })
            }),
        })
    }
}
