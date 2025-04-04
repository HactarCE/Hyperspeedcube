use rhai::packages::{Package, StandardPackage};
use rhai::plugin::*;
use rhai::{Dynamic, EvalAltResult, Module, NativeCallContext, Shared};

mod assertions;
mod operators;
mod types;
mod util;

use crate::Result;

#[derive(Debug, Default, Clone)]
pub struct Point(pub hypermath::Vector);

pub(crate) struct HyperpuzzlePackage(Shared<Module>);
impl HyperpuzzlePackage {
    pub fn new(catalog: &hyperpuzzle_core::Catalog) -> Self {
        let mut module = Module::new();
        Self::init(&mut module);

        module.combine_flatten(exported_module!(assertions::rhai_mod));
        module.combine_flatten(exported_module!(operators::rhai_mod));
        // module.combine_flatten(exported_module!(types::vector::rhai_mod));
        types::register_all_types(&mut module);

        module.build_index();
        Self(Shared::new(module))
    }
}

impl Package for HyperpuzzlePackage {
    fn init(module: &mut Module) {
        StandardPackage::init(module);
        // TODO
    }

    fn as_shared_module(&self) -> Shared<Module> {
        self.0.clone()
    }

    fn init_engine(engine: &mut rhai::Engine) {
        StandardPackage::init_engine(engine);
        engine.set_fast_operators(false);
        engine.set_max_expr_depths(1024, 1024);
    }
}
