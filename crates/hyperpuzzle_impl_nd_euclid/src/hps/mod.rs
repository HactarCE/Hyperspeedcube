//! Hyperpuzzlescript interface for the N-dimensional Euclidean puzzle engine.

use std::fmt;
use std::sync::Arc;

use hypermath::pga::Motor;
use hypermath::{
    ApproxEq, ApproxHash, ApproxInternable, Ndim, Point, Precision, TransformByMotor, Vector,
};
use hyperpuzzle_core::{Axis, NameSpec, Twist, TypedIndex};
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

use axis::{HpsAxis, axis_from_vector, transform_axis};
use axis_system::HpsAxisSystem;
use color::HpsColor;
use layer_mask::HpsLayerMask;
use orbit_names::{HpsOrbitNames, HpsOrbitNamesComponent, Names};
use puzzle::HpsPuzzle;
use region::HpsRegion;
use shape::HpsShape;
use symmetry::HpsSymmetry;
use twist::HpsTwist;
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
            let axis_vectors = object.axes.lock_vectors().at(object_span)?;
            let (id, _) =
                transform_axis(span, &axis_vectors, &transform, (object.id, object_span))?;
            let axes = object.axes.clone();
            HpsAxis { id, axes }
        }
        fn transform(transform: Motor, object: HpsRegion) -> HpsRegion {
            transform.transform(&object)
        }
        fn transform(transform: Motor, object: Names) -> HpsOrbitNames {
            object.0.transform_by(&transform)
        }
        fn transform(transform: Motor, object: HpsSymmetry) -> HpsSymmetry {
            transform.transform(&object)
        }

        fn orbit(ctx: EvalCtx, sym: HpsSymmetry, object: Motor) -> Vec<Spanned<Motor>> {
            symmetry::orbit_spanned(ctx, sym, CanonicalMotor::new(object))
                .into_iter()
                .map(|(CanonicalMotor(m), span)| (m, span))
                .collect()
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
            object.axes.lock_vectors().at(ctx.caller_span)?; // error if vector data is missing
            let vectors = sym.orbit(object.vector().at(object_span)?);
            vectors
                .into_iter()
                .map(|(_, _, v)| {
                    let id = *object.axes.lock_vectors().ok()?.ids_by_vector.get(v)?;
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
            let axes = object.twists.axes();
            axes.lock_vectors().at(ctx.caller_span)?; // error if vector data is missing
            sym.orbit(init_key)
                .iter()
                .map(|(_, _, key)| {
                    let axis = *axes
                        .lock_vectors()
                        .ok()?
                        .ids_by_vector
                        .get(key.axis_vector.clone())?;
                    let id = object
                        .twists
                        .key_to_id(TwistKey::new(axis, &key.transform)?)
                        .ok()??;
                    let twists = object.twists.clone();
                    Some(HpsTwist {
                        id,
                        multiplier: object.multiplier,
                        twists,
                    })
                })
                .map(|opt| (opt, ctx.caller_span))
                .collect()
        }
    ])?;

    Ok(())
}

#[derive(Debug, Clone)]
struct CanonicalMotor(Motor);
impl CanonicalMotor {
    pub fn new(m: Motor) -> Self {
        Self(m.canonicalize_up_to_180().unwrap_or(m))
    }
}
impl Ndim for CanonicalMotor {
    fn ndim(&self) -> u8 {
        self.0.ndim()
    }
}
impl TransformByMotor for CanonicalMotor {
    fn transform_by(&self, m: &Motor) -> Self {
        Self::new(self.0.transform_by(m))
    }
}
impl ApproxEq for CanonicalMotor {
    fn approx_eq(&self, other: &Self, prec: Precision) -> bool {
        prec.eq(&self.0, &other.0)
    }
}
impl ApproxInternable for CanonicalMotor {
    fn intern_floats<F: FnMut(&mut f64)>(&mut self, f: &mut F) {
        self.0.intern_floats(f);
    }
}
impl ApproxHash for CanonicalMotor {
    fn interned_eq(&self, other: &Self) -> bool {
        self.0.interned_eq(&other.0)
    }

    fn interned_hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.interned_hash(state);
    }
}

impl HpsPuzzle {
    fn shape(&self) -> HpsShape {
        HpsShape(Arc::clone(&self.lock().shape))
    }
    fn twists(&self) -> HpsTwistSystem {
        HpsTwistSystem(self.lock().twists.clone())
    }
    fn axes(&self) -> HpsAxisSystem {
        HpsAxisSystem(self.lock().twists.clone())
    }
}
impl HpsTwistSystem {
    fn axes(&self) -> HpsAxisSystem {
        HpsAxisSystem(self.0.clone())
    }
}

fn fmt_puzzle_element(
    f: &mut fmt::Formatter<'_>,
    array_name: &str,
    name: Option<NameSpec>,
    id: impl TypedIndex,
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
        None => write!(f, "{array_name}[{}]", id.to_index()),
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
