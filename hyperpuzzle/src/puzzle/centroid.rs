use std::fmt;
use std::iter::Sum;
use std::ops::{Add, AddAssign};

use hypermath::prelude::*;

/// Centroid and Lebasgue measure of a polytope. In simpler terms: the "center
/// of mass" and "N-dimensional mass" of a polytope.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct Centroid {
    /// Center of mass.
    weighted_center: Vector,
    /// Lebasgue measure (https://w.wiki/FLd), A.K.A. volume.
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
    pub const ZERO: Self = Centroid {
        weighted_center: Vector::EMPTY,
        weight: 0.0,
    };

    pub fn new(center: &Vector, weight: Float) -> Self {
        Centroid {
            weighted_center: center * weight,
            weight,
        }
    }
    pub fn center(&self) -> Vector {
        if is_approx_positive(&self.weight) {
            &self.weighted_center / self.weight
        } else {
            Vector::EMPTY
        }
    }
}
