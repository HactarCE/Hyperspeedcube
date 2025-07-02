//! Hyperpuzzlescript interface for the N-dimensional Euclidean puzzle engine.

use std::fmt;
use std::sync::Arc;

use hypermath::pga::Motor;
use hypermath::{IndexNewtype, Point, Vector};
use hyperpuzzle_core::{Axis, NameSpec, Twist};
use hyperpuzzlescript::{Builtins, ErrorExt, Spanned, hps_fns};
use parking_lot::{Mutex, MutexGuard};

use crate::TwistKey;

mod axis;
mod axis_system;
mod color;
mod layer_mask;
mod orbit_names;
mod puzzle;
mod puzzle_engine;
mod region;
mod shape;
mod symmetry;
mod twist;
mod twist_system;
mod twist_system_engine;

use axis::{HpsAxis, axis_from_vector, axis_name, transform_axis};
use axis_system::HpsAxisSystem;
use color::HpsColor;
use layer_mask::HpsLayerMask;
use orbit_names::{HpsOrbitNames, HpsOrbitNamesComponent, Names};
use puzzle::HpsPuzzle;
use region::HpsRegion;
use shape::HpsShape;
use symmetry::HpsSymmetry;
use twist::{HpsTwist, transform_twist, twist_name};
use twist_system::{GeometricTwistKey, HpsTwistSystem};

/// Hyperpuzzlescript interface for the N-dimensional Euclidean puzzle engine.
///
/// This implements [`hyperpuzzlescript::EngineCallback`].
pub struct HpsNdEuclid;
impl fmt::Display for HpsNdEuclid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "euclid")
    }
}

/// Adds the built-ins.
pub fn define_in(builtins: &mut Builtins<'_>) -> hyperpuzzlescript::Result<()> {
    axis::define_in(builtins)?;
    axis_system::define_in(builtins)?;
    color::define_in(builtins)?;
    orbit_names::define_in(builtins)?;
    puzzle::define_in(builtins)?;
    region::define_in(builtins)?;
    shape::define_in(builtins)?;
    symmetry::define_in(builtins)?;
    twist::define_in(builtins)?;
    twist_system::define_in(builtins)?;

    builtins.set_fns(hps_fns![
        fn transform(ctx: EvalCtx, transform: Motor, (object, object_span): HpsAxis) -> HpsAxis {
            let span = ctx.caller_span;
            let axes = object.axes.lock();
            let id = transform_axis(span, &axes, &transform, (object.id, object_span))?;
            let axes = object.axes.clone();
            HpsAxis { id, axes }
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
        fn transform(transform: Motor, object: Names) -> HpsOrbitNames {
            object.0.transform_by(transform)
        }
        fn transform(transform: Motor, object: HpsSymmetry) -> HpsSymmetry {
            transform.transform(&object)
        }

        fn orbit(ctx: EvalCtx, sym: HpsSymmetry, object: Motor) -> Vec<Spanned<Motor>> {
            symmetry::orbit_spanned(ctx, sym, object)
        }
        fn orbit(ctx: EvalCtx, sym: HpsSymmetry, object: Vector) -> Vec<Spanned<Vector>> {
            symmetry::orbit_spanned(ctx, sym, object)
        }
        fn orbit(ctx: EvalCtx, sym: HpsSymmetry, object: Point) -> Vec<Spanned<Point>> {
            symmetry::orbit_spanned(ctx, sym, object)
        }
        fn orbit(ctx: EvalCtx, sym: HpsSymmetry, object: HpsRegion) -> Vec<Spanned<HpsRegion>> {
            symmetry::orbit_spanned(ctx, sym, object)
        }
        fn orbit(
            ctx: EvalCtx,
            sym: HpsSymmetry,
            (object, object_span): HpsAxis,
        ) -> Vec<Spanned<Option<HpsAxis>>> {
            let vectors = sym.orbit(object.vector().at(object_span)?);
            let axes = object.axes.lock();
            vectors
                .into_iter()
                .map(|(_, _, v)| {
                    let id = axes.vector_to_id(&v)?;
                    let axes = object.axes.clone();
                    Some(HpsAxis { id, axes })
                })
                .map(|opt| (opt, ctx.caller_span))
                .collect()
        }
        fn orbit(
            ctx: EvalCtx,
            sym: HpsSymmetry,
            (object, object_span): HpsTwist,
        ) -> Vec<Spanned<Option<HpsTwist>>> {
            let init_key = GeometricTwistKey {
                axis_vector: object.axis().at(object_span)?.vector().at(object_span)?,
                transform: object.transform().at(object_span)?,
            };
            let twists = object.twists.lock();
            sym.orbit(init_key)
                .iter()
                .map(|(_, _, key)| {
                    let id = twists.key_to_id(&TwistKey::new(
                        twists.axes.vector_to_id(&key.axis_vector)?,
                        &key.transform,
                    )?)?;
                    let twists = object.twists.clone();
                    Some(HpsTwist { id, twists })
                })
                .map(|opt| (opt, ctx.caller_span))
                .collect()
        }
    ])?;

    Ok(())
}

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

impl HpsPuzzle {
    fn shape(&self) -> HpsShape {
        ArcMut(Arc::clone(&self.lock().shape))
    }
    fn twists(&self) -> HpsTwistSystem {
        ArcMut(Arc::clone(&self.lock().twists))
    }
    fn axes(&self) -> HpsAxisSystem {
        HpsAxisSystem(ArcMut(Arc::clone(&self.lock().twists)))
    }
}
impl HpsTwistSystem {
    fn axes(&self) -> HpsAxisSystem {
        HpsAxisSystem(self.clone())
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
        None => write!(f, "{array_name}[{}]", id.to_u64()),
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
