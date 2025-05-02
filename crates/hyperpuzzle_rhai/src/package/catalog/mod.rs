//! Functions for adding objects to the puzzle catalog.

use hyperpuzzle_core::prelude::*;

use super::*;
pub use axis_system::RhaiAxisSystem;
pub use puzzle::RhaiPuzzle;
pub use twist_system::RhaiTwistSystem;

mod axis_system;
mod color_system;
mod cut;
mod generator;
mod puzzle;
mod twist_system;

pub fn init_engine(engine: &mut Engine) {
    axis_system::init_engine(engine);
    puzzle::init_engine(engine);
    twist_system::init_engine(engine);
}

pub fn register(module: &mut Module, catalog: &Catalog, eval_tx: RhaiEvalRequestTx) {
    axis_system::register(module);
    color_system::register(module, catalog, &eval_tx);
    puzzle::register(module, catalog, &eval_tx);
    twist_system::register(module, catalog, &eval_tx);
}
