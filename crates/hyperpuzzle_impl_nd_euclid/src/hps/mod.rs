//! Hyperpuzzlescript interface for the N-dimensional Euclidean puzzle engine.

use std::sync::Arc;

use hypermath::{Point, Vector, pga::Motor};
use hyperpuzzle_core::{Axis, NameSpec, Twist};
use hyperpuzzlescript::{ErrorExt, hps_fns};
use parking_lot::{Mutex, MutexGuard};

use crate::{
    TwistKey,
    builder::{AxisSystemBuilder, PuzzleBuilder, ShapeBuilder, TwistSystemBuilder},
    hps::impl_puzzle_builder::Names,
};

mod axis;
mod color;
mod impl_puzzle_builder;
mod orbit_names;
mod symmetry;
mod twist;

// use name_strategy::{HpsNameFn, HpsNameStrategy};
use axis::HpsAxis;
use color::HpsColor;
use orbit_names::{HpsOrbitNames, HpsOrbitNamesComponent};
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

    scope.register_builtin_functions(hps_fns![
        fn transform(ctx: EvalCtx, transform: Motor, (object, object_span): HpsAxis) -> HpsAxis {
            let v = object.vector().at(object_span)?;
            let id = axis_from_vector(&object.twists.lock().axes, &transform.transform(&v))
                .at(ctx.caller_span)?;
            HpsAxis {
                id,
                twists: object.twists.clone(),
            }
        }
        fn transform(transform: Motor, object: HpsTwist) -> HpsTwist {
            todo!("transform twist")
        }
        fn transform(transform: Motor, object: Names) -> HpsOrbitNames {
            todo!("transform names")
        }
        fn transform(transform: Motor, object: HpsSymmetry) -> HpsSymmetry {
            todo!("transform symmetry")
        }
    ])?;

    Ok(())
}

/// HPS puzzle builder.
type HpsPuzzle = ArcMut<PuzzleBuilder>;
/// HPS twist system builder.
type HpsTwistSystem = ArcMut<TwistSystemBuilder>;
/// HPS shape builder.
type HpsShape = ArcMut<ShapeBuilder>;

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
    fn shape(&self) -> HpsShape {
        ArcMut(Arc::clone(&self.lock().shape))
    }
    fn twists(&self) -> HpsTwistSystem {
        ArcMut(Arc::clone(&self.lock().twists))
    }
}

fn axis_from_vector(axes: &AxisSystemBuilder, vector: &Vector) -> Result<Axis, OrbitNamesError> {
    axes.vector_to_id(&vector)
        .ok_or_else(|| OrbitNamesError::NoAxis(vector.clone()))
}

fn axis_name_from_vector<'a>(
    axes: &'a AxisSystemBuilder,
    vector: &Vector,
) -> Result<&'a NameSpec, OrbitNamesError> {
    let id = axis_from_vector(axes, vector)?;
    axes.names
        .get(id)
        .ok_or_else(|| OrbitNamesError::UnnamedAxis(id, vector.clone()))
}

fn twist_name_from_key<'a>(
    twists: &'a TwistSystemBuilder,
    key: &TwistKey,
) -> Result<&'a NameSpec, OrbitNamesError> {
    let id = twists
        .key_to_id(key)
        .ok_or_else(|| OrbitNamesError::NoTwist(key.clone()))?;
    twists
        .names
        .get(id)
        .ok_or_else(|| OrbitNamesError::UnnamedTwist(id, key.clone()))
}

#[derive(thiserror::Error, Debug, Clone)]
enum OrbitNamesError {
    #[error("no axis with vector {0}")]
    NoAxis(Vector),
    #[error("axis {0} with vector {1} has no name")]
    UnnamedAxis(Axis, Vector),
    #[error("no {0}")]
    NoTwist(TwistKey),
    #[error("{0} has no name")]
    UnnamedTwist(Twist, TwistKey),
    #[error("bad twist transform")]
    BadTwistTransform,
    #[error("missing coset {0}")]
    MissingCoset(Point),
}
