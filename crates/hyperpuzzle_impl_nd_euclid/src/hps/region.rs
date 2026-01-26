use std::fmt;
use std::hash::Hash;
use std::ops::{BitAnd, BitOr, Not};

use hypermath::prelude::*;
use hyperpuzzle_core::LayerMask;
use hyperpuzzlescript::{Builtins, ND_EUCLID, Result, TryEq, hps_fns, impl_simple_custom_type};
use itertools::Itertools;

use super::{HpsAxis, HpsLayerMask, HpsPuzzle};

/// Region of space, typically defined by intersections, unions, and complements
/// of grips.
#[derive(Default, Clone)]
pub enum HpsRegion {
    /// Region containing nothing.
    #[default]
    None,
    /// Region containing all of space.
    All,
    /// Region bounded by a hyperplane.
    HalfSpace(Hyperplane),
    /// Intersection of regions.
    And(Vec<HpsRegion>),
    /// Union of regions.
    Or(Vec<HpsRegion>),
    /// Complement of a region.
    Not(Box<HpsRegion>),
}
impl_simple_custom_type!(HpsRegion = "euclid.Region");
impl fmt::Debug for HpsRegion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self}")
    }
}
impl fmt::Display for HpsRegion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::None => write!(f, "euclid.REGION_NONE"),
            Self::All => write!(f, "euclid.REGION_ALL"),
            Self::HalfSpace(hyperplane) => write!(f, "{ND_EUCLID}.plane({hyperplane}).region()"),
            Self::And(args) => write!(f, "({})", args.iter().join(" & ")),
            Self::Or(args) => write!(f, "({})", args.iter().join(" | ")),
            Self::Not(arg) => write!(f, "~{arg}"),
        }
    }
}
impl TryEq for HpsRegion {
    fn try_eq(&self, _other: &Self) -> Option<bool> {
        None // fail
    }
}

/// Adds the built-ins.
pub fn define_in(builtins: &mut Builtins<'_>) -> Result<()> {
    builtins.set_custom_ty::<HpsRegion>()?;

    builtins.set("euclid.REGION_NONE", HpsRegion::None)?;
    builtins.set("euclid.REGION_ALL", HpsRegion::All)?;

    builtins.set_fns(hps_fns![
        fn region(plane: Hyperplane) -> HpsRegion {
            HpsRegion::HalfSpace(plane)
        }

        fn region(ctx: EvalCtx, puzzle: HpsPuzzle, axis: HpsAxis) -> HpsRegion {
            let layer_count = puzzle.lock().axis_layers(axis.id).0.len();
            let layer_mask = LayerMask::all_layers(layer_count as u8);
            puzzle.layer_regions(ctx, axis.id, layer_mask)?
        }
        fn region(
            ctx: EvalCtx,
            puzzle: HpsPuzzle,
            axis: HpsAxis,
            layer_mask: HpsLayerMask,
        ) -> HpsRegion {
            puzzle.layer_regions(ctx, axis.id, layer_mask.0)?
        }

        fn contains(region: HpsRegion, point: Point) -> bool {
            region.contains_point(&point)
        }

        fn union(regions: Vec<HpsRegion>) -> HpsRegion {
            regions
                .into_iter()
                .reduce(|a, b| a | b)
                .unwrap_or(HpsRegion::None)
        }
        fn intersect(regions: Vec<HpsRegion>) -> HpsRegion {
            regions
                .into_iter()
                .reduce(|a, b| a & b)
                .unwrap_or(HpsRegion::All)
        }
    ])?;

    builtins.set_fns(hps_fns![
        ("&", |_, a: HpsRegion, b: HpsRegion| -> HpsRegion { a & b }),
        ("|", |_, a: HpsRegion, b: HpsRegion| -> HpsRegion { a | b }),
        ("~", |_, r: HpsRegion| -> HpsRegion { !r }),
    ])
}

impl HpsRegion {
    /// Returns whether the region contains a point. If the point is
    /// approximately on the region boundary, it is considered inside the
    /// region.
    pub fn contains_point(&self, point: &Point) -> bool {
        match self {
            HpsRegion::None => false,
            HpsRegion::All => true,
            HpsRegion::HalfSpace(h) => match h.location_of_point(point) {
                hypermath::PointWhichSide::On => true,
                hypermath::PointWhichSide::Inside => true,
                hypermath::PointWhichSide::Outside => false,
            },
            HpsRegion::And(xs) => xs.iter().all(|x| x.contains_point(point)),
            HpsRegion::Or(xs) => xs.iter().any(|x| x.contains_point(point)),
            HpsRegion::Not(x) => !x.contains_point(point),
        }
    }
}

impl Ndim for HpsRegion {
    fn ndim(&self) -> u8 {
        match self {
            HpsRegion::None => 0,
            HpsRegion::All => 0,
            HpsRegion::HalfSpace(h) => h.normal().ndim(),
            HpsRegion::And(xs) | HpsRegion::Or(xs) => {
                xs.iter().map(|x| x.ndim()).max().unwrap_or(0)
            }
            HpsRegion::Not(x) => x.ndim(),
        }
    }
}

impl TransformByMotor for HpsRegion {
    fn transform_by(&self, m: &hypermath::pga::Motor) -> Self {
        match self {
            Self::None => Self::None,
            Self::All => Self::All,
            Self::HalfSpace(h) => Self::HalfSpace(m.transform(h)),
            Self::And(xs) => Self::And(xs.iter().map(|x| m.transform(x)).collect()),
            Self::Or(xs) => Self::Or(xs.iter().map(|x| m.transform(x)).collect()),
            Self::Not(x) => Self::Not(Box::new(m.transform(x))),
        }
    }
}

impl ApproxEq for HpsRegion {
    fn approx_eq(&self, other: &Self, prec: Precision) -> bool {
        match (self, other) {
            (HpsRegion::None, HpsRegion::None) => true,
            (HpsRegion::All, HpsRegion::All) => true,
            (HpsRegion::HalfSpace(h1), HpsRegion::HalfSpace(h2)) => prec.eq(h1, h2),
            (HpsRegion::And(r1), HpsRegion::And(r2)) => prec.eq(r1, r2),
            (HpsRegion::Or(r1), HpsRegion::Or(r2)) => prec.eq(r1, r2),
            (HpsRegion::Not(r1), HpsRegion::Not(r2)) => prec.eq(r1, r2),

            (HpsRegion::None, _) | (_, HpsRegion::None) => false,
            (HpsRegion::All, _) | (_, HpsRegion::All) => false,
            (HpsRegion::HalfSpace(_), _) | (_, HpsRegion::HalfSpace(_)) => false,
            (HpsRegion::And(_), _) | (_, HpsRegion::And(_)) => false,
            (HpsRegion::Or(_), _) | (_, HpsRegion::Or(_)) => false,
        }
    }
}

impl ApproxHash for HpsRegion {
    fn intern_floats<F: FnMut(&mut f64)>(&mut self, f: &mut F) {
        match self {
            HpsRegion::None | HpsRegion::All => (),
            HpsRegion::HalfSpace(h) => h.intern_floats(f),
            HpsRegion::And(r) | HpsRegion::Or(r) => r.intern_floats(f),
            HpsRegion::Not(r) => r.intern_floats(f),
        }
    }

    fn interned_eq(&self, other: &Self) -> bool {
        match (self, other) {
            (HpsRegion::None, HpsRegion::None) => true,
            (HpsRegion::All, HpsRegion::All) => true,
            (HpsRegion::HalfSpace(h1), HpsRegion::HalfSpace(h2)) => h1.interned_eq(h2),
            (HpsRegion::And(r1), HpsRegion::And(r2)) => r1.interned_eq(r2),
            (HpsRegion::Or(r1), HpsRegion::Or(r2)) => r1.interned_eq(r2),
            (HpsRegion::Not(r1), HpsRegion::Not(r2)) => r1.interned_eq(r2),

            (HpsRegion::None, _)
            | (HpsRegion::All, _)
            | (HpsRegion::HalfSpace(_), _)
            | (HpsRegion::And(_), _)
            | (HpsRegion::Or(_), _)
            | (HpsRegion::Not(_), _) => false,
        }
    }

    fn interned_hash<H: std::hash::Hasher>(&self, state: &mut H) {
        std::mem::discriminant(self).hash(state);
        match self {
            HpsRegion::None | HpsRegion::All => (),
            HpsRegion::HalfSpace(hyperplane) => hyperplane.interned_hash(state),
            HpsRegion::And(hps_regions) => hps_regions.interned_hash(state),
            HpsRegion::Or(hps_regions) => hps_regions.interned_hash(state),
            HpsRegion::Not(hps_region) => hps_region.interned_hash(state),
        }
    }
}

impl BitAnd for HpsRegion {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (Self::None, _) | (_, Self::None) => Self::None,
            (Self::All, other) | (other, Self::All) => other,
            (Self::And(mut xs), Self::And(ys)) => {
                xs.extend(ys);
                Self::And(xs)
            }
            (Self::And(mut xs), y) | (y, Self::And(mut xs)) => {
                xs.push(y);
                Self::And(xs)
            }
            (x, y) => Self::And(vec![x, y]),
        }
    }
}
impl BitOr for HpsRegion {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (Self::All, _) | (_, Self::All) => Self::All,
            (Self::None, other) | (other, Self::None) => other,
            (Self::Or(mut xs), Self::Or(ys)) => {
                xs.extend(ys);
                Self::Or(xs)
            }
            (Self::Or(mut xs), y) | (y, Self::Or(mut xs)) => {
                xs.push(y);
                Self::Or(xs)
            }
            (x, y) => Self::Or(vec![x, y]),
        }
    }
}
impl Not for HpsRegion {
    type Output = Self;

    fn not(self) -> Self::Output {
        match self {
            Self::None => Self::All,
            Self::All => Self::None,
            Self::Not(inner) => *inner,
            other => Self::Not(Box::new(other)),
        }
    }
}
