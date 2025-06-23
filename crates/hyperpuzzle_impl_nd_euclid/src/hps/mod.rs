//! Hyperpuzzlescript interface for the N-dimensional Euclidean puzzle engine.

use std::sync::Arc;

use parking_lot::{Mutex, MutexGuard};

use crate::builder::{PuzzleBuilder, ShapeBuilder, TwistSystemBuilder};

mod axis;
mod color;
mod impl_puzzle_builder;
mod orbit_names;
mod symmetry;
mod twist;

// use name_strategy::{HpsNameFn, HpsNameStrategy};
use axis::HpsAxis;
use color::HpsColor;
use orbit_names::HpsOrbitNames;
use symmetry::HpsSymmetry;
use twist::HpsTwist;

/// Hyperpuzzlescript interface for the N-dimensional Euclidean puzzle engine.
pub struct HpsNdEuclid;

/// Adds the built-ins to the scope.
pub fn define_in(scope: &hyperpuzzlescript::Scope) -> hyperpuzzlescript::Result<()> {
    scope.register_custom_type::<HpsAxis>();
    scope.register_custom_type::<HpsColor>();
    impl_puzzle_builder::define_in(scope)?;
    orbit_names::define_in(scope)?;
    symmetry::define_in(scope)?;
    scope.register_custom_type::<HpsTwist>();
    Ok(())
}

/// HPS puzzle builder.
type HpsPuzzleBuilder = ArcMut<PuzzleBuilder>;
/// HPS twist system builder.
type HpsTwistSystem = ArcMut<TwistSystemBuilder>;
/// HPS shape builder.
type HpsShapeBuilder = ArcMut<ShapeBuilder>;

/// Shared mutable wrapper for HPS builder types.
#[derive(Default)]
struct ArcMut<T>(Arc<Mutex<T>>);
impl<T> PartialEq for ArcMut<T> {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}
impl<T> Eq for ArcMut<T> {}
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
