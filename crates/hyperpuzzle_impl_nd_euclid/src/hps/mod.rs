//! Hyperpuzzlescript interface for the N-dimensional Euclidean puzzle engine.

use std::fmt;
use std::sync::Arc;

use hypermath::pga::Motor;
use hypermath::{IndexNewtype, Point, Vector};
use hyperpuzzle_core::{Axis, NameSpec, Twist};
use hyperpuzzlescript::{Builtins, ErrorExt, hps_fns};
use parking_lot::{Mutex, MutexGuard};

use crate::TwistKey;
use crate::builder::{PuzzleBuilder, ShapeBuilder, TwistSystemBuilder};

mod axis;
mod color;
mod impl_puzzle_builder;
mod impl_twist_system_builder;
mod orbit_names;
mod region;
mod symmetry;
mod twist;

use axis::{HpsAxis, axis_from_vector, axis_name, transform_axis};
use color::HpsColor;
use orbit_names::{HpsOrbitNames, HpsOrbitNamesComponent, Names};
use region::HpsRegion;
use symmetry::HpsSymmetry;
use twist::{HpsTwist, transform_twist, twist_name};

/// Hyperpuzzlescript interface for the N-dimensional Euclidean puzzle engine.
pub struct HpsNdEuclid;
impl fmt::Display for HpsNdEuclid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "euclid")
    }
}

/// Adds the built-ins.
pub fn define_in(builtins: &mut Builtins<'_>) -> hyperpuzzlescript::Result<()> {
    builtins.set_custom_ty::<HpsAxis>()?;
    builtins.set_custom_ty::<HpsColor>()?;
    impl_puzzle_builder::define_in(builtins)?;
    impl_twist_system_builder::define_in(builtins)?;
    orbit_names::define_in(builtins)?;
    region::define_in(builtins)?;
    symmetry::define_in(builtins)?;
    builtins.set_custom_ty::<HpsTwist>()?;

    builtins.set_fns(hps_fns![
        fn transform(ctx: EvalCtx, transform: Motor, (object, object_span): HpsAxis) -> HpsAxis {
            let span = ctx.caller_span;
            let twists = object.twists.lock();
            let id = transform_axis(span, &twists.axes, &transform, (object.id, object_span))?;
            drop(twists);
            let twists = object.twists;
            HpsAxis { id, twists }
        }
        fn transform(ctx: EvalCtx, transform: Motor, (object, object_span): HpsTwist) -> HpsTwist {
            let span = ctx.caller_span;
            let twists = object.twists.lock();
            let id = transform_twist(span, &twists, &transform, (object.id, object_span))?;
            drop(twists);
            let twists = object.twists;
            HpsTwist { id, twists }
        }
        fn transform(transform: Motor, object: HpsRegion) -> HpsRegion {
            transform.transform(&object)
        }
        // fn transform(transform: Motor, object: Names) -> HpsOrbitNames {
        //     todo!("transform names")
        // }
        // fn transform(transform: Motor, object: HpsSymmetry) -> HpsSymmetry {
        //     todo!("transform symmetry")
        // }

        fn rev(ctx: EvalCtx, twist: HpsTwist) -> Option<HpsTwist> {
            let rev_id = twist.twists.lock().inverse(twist.id).at(ctx.caller_span)?;
            rev_id.map(|id| HpsTwist {
                id,
                twists: twist.twists.clone(),
            })
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

fn fmt_puzzle_element(
    f: &mut fmt::Formatter<'_>,
    array_name: &str,
    name: Option<NameSpec>,
    id: impl IndexNewtype,
) -> fmt::Result {
    match name {
        Some(name) => {
            let k = hyperpuzzlescript::codegen::to_map_key(&name.preferred);
            if k.starts_with('"') {
                write!(f, "{array_name}[{k}]")
            } else {
                write!(f, "{array_name}.{k}")
            }
        }
        None => write!(f, "{array_name}[{}]", id),
    }
}

#[derive(thiserror::Error, Debug, Clone)]
enum HpsEuclidError {
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
