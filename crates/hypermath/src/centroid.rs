//! Centroid and Lebasgue measure.

use std::fmt;
use std::iter::Sum;
use std::ops::{Add, AddAssign};

use super::{Float, Vector, approx_eq, is_approx_positive};

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

impl Centroid {
    /// Zero centroid.
    pub const ZERO: Self = Centroid {
        weighted_center: Vector::EMPTY,
        weight: 0.0,
    };

    /// Constructs a new weighted centroid.
    pub fn new(center: &Vector, weight: Float) -> Self {
        Centroid {
            weighted_center: center * weight,
            weight,
        }
    }
    /// Returns the centroid point.
    pub fn center(&self) -> Vector {
        if is_approx_positive(&self.weight) {
            &self.weighted_center / self.weight
        } else {
            Vector::EMPTY
        }
    }
    /// Returns the weight.
    pub fn weight(&self) -> Float {
        self.weight
    }
    /// Returns whether the weight is zero.
    pub fn is_zero(&self) -> bool {
        approx_eq(&self.weight, &0.0)
    }
}
