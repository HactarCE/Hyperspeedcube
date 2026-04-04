//! Centroid and Lebasgue measure.

use std::fmt;
use std::iter::Sum;
use std::ops::{Add, AddAssign};

use approx_collections::{ApproxHash, ApproxInternable};

use crate::{APPROX, Float, Ndim, Point, TransformByMotor, Vector};

/// Centroid and Lebasgue measure of a polytope element. In simpler terms: the
/// "center of mass" and "N-dimensional mass" of a polytope element.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct Centroid {
    /// Center of mass, scaled by `weight`.
    weighted_center: Vector,
    /// [Lebasgue measure](https://w.wiki/FLd), a.k.a. volume.
    weight: Float,
}

impl fmt::Display for Centroid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl Add for Centroid {
    type Output = Self;

    fn add(mut self, rhs: Self) -> Self::Output {
        self += rhs;
        self
    }
}

impl AddAssign<&Centroid> for Centroid {
    fn add_assign(&mut self, rhs: &Centroid) {
        self.weighted_center += &rhs.weighted_center;
        self.weight += rhs.weight;
    }
}

impl AddAssign<Centroid> for Centroid {
    fn add_assign(&mut self, rhs: Centroid) {
        *self += &rhs;
    }
}

impl Sum<Centroid> for Centroid {
    fn sum<I: Iterator<Item = Centroid>>(mut iter: I) -> Self {
        let mut ret = iter.next().unwrap_or_default();
        for it in iter {
            ret += it;
        }
        ret
    }
}

impl Ndim for Centroid {
    fn ndim(&self) -> u8 {
        self.weighted_center.ndim()
    }
}

impl TransformByMotor for Centroid {
    fn transform_by(&self, m: &crate::pga::Motor) -> Self {
        Self::new(&m.transform(&self.center()), self.weight)
    }
}

impl ApproxInternable for Centroid {
    fn intern_floats<F: FnMut(&mut f64)>(&mut self, f: &mut F) {
        self.weighted_center.intern_floats(f);
        self.weight.intern_floats(f);
    }
}

impl ApproxHash for Centroid {
    fn interned_eq(&self, other: &Self) -> bool {
        self.weighted_center.interned_eq(&other.weighted_center)
            && self.center().interned_eq(&other.center())
    }

    fn interned_hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.weighted_center.interned_hash(state);
        self.weight.interned_hash(state);
    }
}

impl Centroid {
    /// Zero centroid.
    pub const ZERO: Self = Centroid {
        weighted_center: Vector::EMPTY,
        weight: 0.0,
    };

    /// Constructs a new weighted centroid.
    pub fn new(center: &Point, weight: Float) -> Self {
        Centroid {
            weighted_center: center.as_vector() * weight,
            weight,
        }
    }
    /// Returns the centroid point.
    pub fn center(&self) -> Point {
        if APPROX.is_pos(self.weight) {
            Point(&self.weighted_center / self.weight)
        } else {
            Point::ORIGIN
        }
    }
    /// Returns the weight.
    pub fn weight(&self) -> Float {
        self.weight
    }
    /// Returns whether the weight is zero.
    pub fn is_zero(&self) -> bool {
        APPROX.eq_zero(self.weight)
    }
}
