use rhai::packages::{Package, StandardPackage};
use rhai::plugin::*;
use rhai::{Dynamic, EvalAltResult, FnPtr, Map, Module, NativeCallContext, Shared};

mod assertions;
mod catalog;
mod geometry;
mod operators;
mod orbit_names;
mod state;
mod syntax;
mod types;

use state::RhaiState;

use crate::convert::*;
use crate::errors::*;
use crate::loader::RhaiEvalRequestTx;
use crate::util::{new_fn, void_warn, warn, warnf};
use crate::{Ctx, Result, RhaiCtx};
pub use types::elements::{LockAs, RhaiAxis, RhaiColor, RhaiPuzzleElement, RhaiTwist};

pub(crate) struct HyperpuzzlePackage(Shared<Module>);
impl HyperpuzzlePackage {
    pub fn new(catalog: &hyperpuzzle_core::Catalog, eval_tx: RhaiEvalRequestTx) -> Self {
        let mut module = Module::new();
        Self::init(&mut module);

        assertions::register(&mut module);
        catalog::register(&mut module, catalog, eval_tx);
        geometry::register(&mut module);
        operators::register(&mut module);
        types::register(&mut module);

        module.set_var("PI", std::f64::consts::PI);
        module.set_var("TAU", std::f64::consts::TAU);
        module.set_var("PHI", (1.0 + f64::sqrt(5.0)) * 0.5);
        new_fn("deg").set_into_module(&mut module, |deg: i64| {
            deg as f64 * std::f64::consts::PI / 180.0
        });
        new_fn("deg").set_into_module(&mut module, |deg: f64| deg * std::f64::consts::PI / 180.0);

        module.build_index();
        Self(Shared::new(module))
    }
}

impl Package for HyperpuzzlePackage {
    fn init(module: &mut Module) {
        StandardPackage::init(module);
    }

    fn as_shared_module(&self) -> Shared<Module> {
        self.0.clone()
    }

    fn init_engine(engine: &mut rhai::Engine) {
        StandardPackage::init_engine(engine);
        engine.set_fast_operators(false);
        engine.set_max_expr_depths(1024, 1024);
        catalog::init_engine(engine);
        state::init_engine(engine);
        syntax::init_engine(engine);
        types::init_engine(engine);
    }
}
