use std::sync::Arc;

use eyre::eyre;
use hyperpuzzle_core::prelude::*;
use hyperpuzzlescript::*;

use super::{ArcMut, HpsNdEuclid};
use crate::builder::*;

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
    ) -> Result<LazyCatalogConstructor<TwistSystem>> {
        let caller_span = ctx.caller_span;

        unpack_kwargs!(kwargs, ndim: u8, (build, build_span): Arc<FnValue>);

        let meta = Arc::new(meta);

        Ok(LazyCatalogConstructor {
            meta: Arc::clone(&meta),
            build: Box::new(move |build_ctx| {
                let id = meta.id.clone();
                let builder = ArcMut::new(TwistSystemBuilder::new_shared(
                    id.clone(),
                    Some(meta.name.clone()),
                    ndim,
                ));

                let mut scope = Scope::default();
                scope.special.ndim = Some(ndim);
                scope.special.twists = builder.clone().at(BUILTIN_SPAN);
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
                    let exports = build_fn
                        .call(build_span, &mut ctx, vec![], Map::new())
                        .map_err(|e| {
                            ctx.runtime.report_diagnostic(e);
                            eyre!("unable to build twist system `{id}`; see HPS logs")
                        })?;

                    let mut b = builder.lock();
                    if let Ok(exports_map) = exports.to::<Arc<Map>>() {
                        b.hps_exports = exports_map;
                    }
                    b.is_modified = false;

                    let puzzle_id = None;
                    b.build(Some(&build_ctx), puzzle_id, &mut ctx.warnf())
                        .map(Arc::new)
                })
            }),
        })
    }
}
