//! Functions for adding objects to the puzzle catalog.

use hyperpuzzle_core::prelude::*;

use super::*;

mod color_system;

pub fn register(module: &mut Module, catalog: &Catalog) {
    color_system::register(module, catalog);
}
