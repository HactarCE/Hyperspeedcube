use std::sync::Arc;

use hyperpuzzle_core::prelude::*;
use hyperpuzzle_impl_nd_euclid::builder::*;
use parking_lot::Mutex;

mod color;

use super::*;

pub fn init_engine(engine: &mut Engine) {
    color::init_engine(engine);
}

pub fn register(module: &mut Module) {
    color::register(module);
}

/// Rhai handle to a puzzle element, indexed by ID.
#[derive(Clone)]
pub struct RhaiPuzzleElement<I> {
    /// ID of the puzzle element.
    pub id: I,
    /// Underlying database.
    pub db: Arc<Mutex<PuzzleBuilder>>,
}
impl<I: PartialEq> PartialEq for RhaiPuzzleElement<I> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id && Arc::ptr_eq(&self.db, &other.db)
    }
}
impl<I: Eq> Eq for RhaiPuzzleElement<I> {}
