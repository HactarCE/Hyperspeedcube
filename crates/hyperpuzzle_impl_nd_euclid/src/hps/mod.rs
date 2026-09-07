//! Hyperpuzzlescript geometry vocabulary shared by puzzle backends.

use std::fmt;

use hypermath::pga::Motor;
use hypermath::{
    ApproxEq, ApproxHash, ApproxInternable, Ndim, Point, Precision, TransformByMotor, Vector,
};
use hyperpuzzlescript::{Builtins, ErrorExt, Spanned, hps_fns};

mod orbit_names;
mod symmetry;

pub use orbit_names::{ElementNames, HpsOrbitNames, HpsOrbitNamesComponent};
pub use symmetry::HpsSymmetry;

/// Adds the built-ins.
pub fn define_in(builtins: &mut Builtins<'_>) -> hyperpuzzlescript::Result<()> {
    orbit_names::define_in(builtins)?;
    symmetry::define_in(builtins)?;

    builtins.set_fns(hps_fns![
        fn transform(transform: Motor, object: ElementNames) -> HpsOrbitNames {
            object.0.transform_by(&transform)
        }
        fn transform(transform: Motor, object: HpsSymmetry) -> HpsSymmetry {
            transform.transform(&object)
        }

        fn orbit(ctx: EvalCtx, sym: HpsSymmetry, object: Motor) -> Vec<Spanned<Motor>> {
            symmetry::orbit_spanned(ctx, sym, CanonicalMotor::new(object))?
                .into_iter()
                .map(|(CanonicalMotor(m), span)| (m, span))
                .collect()
        }
        fn orbit(ctx: EvalCtx, sym: HpsSymmetry, object: Vector) -> Vec<Spanned<Vector>> {
            symmetry::orbit_spanned(ctx, sym, object)?
        }
        fn orbit(ctx: EvalCtx, sym: HpsSymmetry, object: Point) -> Vec<Spanned<Point>> {
            symmetry::orbit_spanned(ctx, sym, object)?
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

#[derive(thiserror::Error, Debug, Clone)]
pub(super) enum HpsEuclidError {
    #[error("missing coset {0}")]
    MissingCoset(Point),
}
