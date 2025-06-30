use std::fmt;
use std::ops::{BitAnd, BitOr, BitXor, Not};

use hypermath::collections::approx_hashmap::{FloatHash, VectorHash};
use hypermath::{ApproxHashMapKey, Float, Hyperplane, Point, TransformByMotor};
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
    /// Symmetric difference of regions.
    Xor(Vec<HpsRegion>),
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
            Self::Xor(args) => write!(f, "({})", args.iter().join(" ^ ")),
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
        ("^", |_, a: HpsRegion, b: HpsRegion| -> HpsRegion { a ^ b }),
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
            HpsRegion::Xor(xs) => xs.iter().filter(|x| x.contains_point(point)).count() % 2 == 1,
            HpsRegion::Not(x) => !x.contains_point(point),
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
            Self::Xor(xs) => Self::Xor(xs.iter().map(|x| m.transform(x)).collect()),
            Self::Not(x) => Self::Not(Box::new(m.transform(x))),
        }
    }
}

impl ApproxHashMapKey for HpsRegion {
    type Hash = (Vec<VectorHash>, String);

    fn approx_hash(&self, mut float_hash_fn: impl FnMut(Float) -> FloatHash) -> Self::Hash {
        // Hyperplanes that factor into the region.
        let mut planes = vec![];

        // Serialization of the tree of operations to construct the region.
        let mut ast_structure = String::new();

        hash_region(&mut float_hash_fn, &mut planes, &mut ast_structure, self);
        (planes, ast_structure)
    }
}

fn hash_region(
    float_hash_fn: &mut impl FnMut(Float) -> FloatHash,
    planes: &mut Vec<VectorHash>,
    ast_structure: &mut String,
    r: &HpsRegion,
) {
    // The hash needs to be unambigous, but we never have to decode it, so this
    // is essentially a silly little domain-specific language.
    match r {
        HpsRegion::None => ast_structure.push('_'),
        HpsRegion::All => ast_structure.push('*'),
        HpsRegion::HalfSpace(h) => {
            ast_structure.push('h');
            planes.push(h.approx_hash(float_hash_fn));
        }
        HpsRegion::And(xs) => {
            // `&XYZ.` = intersection of X, Y, and Z
            ast_structure.push('&');
            for x in xs {
                hash_region(float_hash_fn, planes, ast_structure, x);
            }
            ast_structure.push('.');
        }
        HpsRegion::Or(xs) => {
            // `|XYZ.` = union of X, Y, and Z
            ast_structure.push('|');
            for x in xs {
                hash_region(float_hash_fn, planes, ast_structure, x);
            }
            ast_structure.push('.');
        }
        HpsRegion::Xor(xs) => {
            // `^XYZ.` = symmetric difference of X, Y, and Z
            ast_structure.push('^');
            for x in xs {
                hash_region(float_hash_fn, planes, ast_structure, x);
            }
            ast_structure.push('.');
        }
        HpsRegion::Not(x) => {
            // `~X` = complement of X
            ast_structure.push('~');
            hash_region(float_hash_fn, planes, ast_structure, x);
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
impl BitXor for HpsRegion {
    type Output = Self;

    fn bitxor(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (HpsRegion::None, x) | (x, HpsRegion::None) => x,
            (HpsRegion::All, x) | (x, HpsRegion::All) => !x,
            (HpsRegion::Xor(mut xs), HpsRegion::Xor(ys)) => {
                xs.extend(ys);
                Self::Xor(xs)
            }
            (HpsRegion::Xor(mut xs), x) | (x, HpsRegion::Xor(mut xs)) => {
                xs.push(x);
                Self::Xor(xs)
            }
            (x, y) => HpsRegion::Xor(vec![x, y]),
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
