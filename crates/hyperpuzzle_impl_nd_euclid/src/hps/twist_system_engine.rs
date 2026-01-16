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

        let IdAndName { id, name } = meta;

        unpack_kwargs!(kwargs, ndim: u8, (build, build_span): Arc<FnValue>);

        Ok(TwistSystemSpec {
            id: id.clone(),
            name: name.clone(),
            build: Box::new(move |build_ctx| {
                dbg!("into the twist system build!");
                let id = id.clone();
                let builder = ArcMut::new(TwistSystemBuilder::new_shared(
                    id.clone(),
                    Some(name.clone()),
                    ndim,
                ));

                dbg!("make scope");
                let mut scope = Scope::default();
                scope.special.ndim = Some(ndim);
                scope.special.twists = builder.clone().at(BUILTIN_SPAN);
                scope.special.axes = builder.axes().at(BUILTIN_SPAN);
                scope.special.id = Some((&id).into());
                let scope = Arc::new(scope);

                let build_fn = Arc::clone(&build);

                dbg!("eval plz");
                eval_tx
                    .eval_blocking(move |runtime| {
                        dbg!("sweet, we got a runtime");
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
                            .map(|ok| Redirectable::Direct(Arc::new(ok)))
                    })
                    .map_err(|e| format!("{e:#}"))
            }),
        })
    }
}
