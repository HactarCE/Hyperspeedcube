//! Functions for adding objects to the puzzle catalog.

use hyperpuzzle_core::prelude::*;

use super::*;

mod color_system;
mod generator;

pub fn register(module: &mut Module, catalog: &Catalog, eval_tx: RhaiEvalRequestTx) {
    color_system::register(module, catalog, &eval_tx);
}
