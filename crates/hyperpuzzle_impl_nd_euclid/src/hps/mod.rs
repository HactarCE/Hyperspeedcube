//! Hyperpuzzlescript interface for the N-dimensional Euclidean puzzle engine.

use std::sync::Arc;

use parking_lot::{Mutex, MutexGuard};

use crate::builder::{PuzzleBuilder, ShapeBuilder, TwistSystemBuilder};

mod impl_puzzle_builder;

/// Hyperpuzzlescript interface for the N-dimensional Euclidean puzzle engine.
pub struct HpsNdEuclid;

/// Shared mutable wrapper for HPS puzzle builder types.
#[derive(Debug, Default)]
struct ArcMut<T>(Arc<Mutex<T>>);
impl<T> Clone for ArcMut<T> {
    fn clone(&self) -> Self {
        Self(Arc::clone(&self.0))
    }
}
impl<T> From<Arc<Mutex<T>>> for ArcMut<T> {
    fn from(value: Arc<Mutex<T>>) -> Self {
        Self(value)
    }
}
impl<T> ArcMut<T> {
    fn new(inner: T) -> Self {
        Self(Arc::new(Mutex::new(inner)))
    }
    fn lock(&self) -> MutexGuard<'_, T> {
        self.0.lock()
    }
}

impl ArcMut<PuzzleBuilder> {
    fn shape(&self) -> Arc<Mutex<ShapeBuilder>> {
        Arc::clone(&self.lock().shape)
    }
    fn twists(&self) -> Arc<Mutex<TwistSystemBuilder>> {
        Arc::clone(&self.lock().twists)
    }
}
