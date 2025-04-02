use rhai::{
    FuncRegistration, Module, Shared,
    packages::{Package, StandardPackage},
};

pub(crate) struct HyperpuzzlePackage(Shared<Module>);
impl HyperpuzzlePackage {
    pub fn new(catalog: &hyperpuzzle_core::Catalog) -> Self {
        let mut module = Module::new();
        Self::init(&mut module);

        // i64 / i64 -> f64
        FuncRegistration::new("/")
            .in_global_namespace()
            .set_into_module(&mut module, |a: i64, b: i64| a as f64 / b as f64);

        // TODO: more functions!

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
