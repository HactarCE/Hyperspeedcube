// Types defined in the Rhai API.

use super::*;

pub mod elements;
pub mod hyperplane;
pub mod point;
pub mod symmetry;
pub mod transform;
pub mod vector;

pub fn init_engine(engine: &mut Engine) {
    elements::init_engine(engine);
    hyperplane::init_engine(engine);
    point::init_engine(engine);
    symmetry::init_engine(engine);
    transform::init_engine(engine);
    vector::init_engine(engine);
}

pub fn register(module: &mut Module) {
    elements::register(module);
    hyperplane::register(module);
    point::register(module);
    symmetry::register(module);
    transform::register(module);
    vector::register(module);
}
