// Types defined in the Rhai API.

use super::*;

mod hyperplane;
mod point;
mod transform;
mod vector;

pub(super) fn register(module: &mut Module) {
    hyperplane::register(module);
    point::register(module);
    transform::register(module);
    vector::register(module);
}

pub(super) fn init_engine(engine: &mut Engine) {
    hyperplane::init_engine(engine);
    point::init_engine(engine);
    transform::init_engine(engine);
    vector::init_engine(engine);
}
