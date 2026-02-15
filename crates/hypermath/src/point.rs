//! N-dimensional Euclidean point.

use std::fmt;
use std::ops::*;

use approx_collections::{ApproxEq, ApproxEqZero, ApproxHash, ApproxInternable, Precision};

use crate::{Float, Ndim, Vector, VectorRef};

/// Constructs an N-dimensional Euclidean point, using the same syntax as
/// `vec![]`.
#[macro_export]
macro_rules! point {
    [$($tok:tt)*] => {
        $crate::Point($crate::vector![$($tok)*])
    };
}

/// N-dimensional Eudlicean point. Indexing out of bounds returns zero.
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(transparent)
)]
#[derive(Debug, Default, Clone, PartialEq)]
pub struct Point(pub Vector);

impl<V: VectorRef> From<V> for Point {
    fn from(value: V) -> Self {
        Point(value.to_vector())
    }
}

impl Ndim for Point {
    /// Returns the number of components in the point.
    fn ndim(&self) -> u8 {
        self.0.ndim()
    }
}

impl Point {
    /// Zero-dimensional origin point.
    pub const ORIGIN: Point = Point(Vector::EMPTY);

    /// Returns the origin point.
    pub fn origin(ndim: u8) -> Point {
        Point(Vector::zero(ndim))
    }

    /// Returns a reference to the components of the point as a vector.
    pub fn as_vector(&self) -> &Vector {
        &self.0
    }
    /// Returns a mutable reference to the components of the point as a
    /// vector.
    pub fn as_vector_mut(&mut self) -> &mut Vector {
        &mut self.0
    }
    /// Returns the vector from the origin to the point.
    pub fn into_vector(self) -> Vector {
        self.0
    }

    /// Projects the point onto `other`.
    pub fn projected_to(&self, other: &Vector) -> Option<Point> {
        self.0.projected_to(other).map(Point)
    }
    /// Projects the point onto the perpendicular space of `other`.
    pub fn rejected_from(&self, other: &Vector) -> Option<Point> {
        self.0.rejected_from(other).map(Point)
    }

    /// Interpolates between `self` and `other` with a value of `other_weight /
    /// (self_weight + other_weight)`, where 0 is `self` and 1 is `other`.
    pub fn normalized_weighted_sum(
        &self,
        self_weight: Float,
        other: &Point,
        other_weight: Float,
    ) -> Point {
        Point((&self.0 * self_weight + &other.0 * other_weight) / (self_weight + other_weight))
    }
}

impl ApproxEq for Point {
    fn approx_eq(&self, other: &Self, prec: Precision) -> bool {
        prec.eq(&self.0, &other.0)
    }
}

impl ApproxEqZero for Point {
    fn approx_eq_zero(&self, prec: Precision) -> bool {
        self.0.approx_eq_zero(prec)
    }
}

impl ApproxInternable for Point {
    fn intern_floats<F: FnMut(&mut f64)>(&mut self, f: &mut F) {
        self.0.intern_floats(f);
    }
}
impl ApproxHash for Point {
    fn interned_eq(&self, other: &Self) -> bool {
        self.0.interned_eq(&other.0)
    }

    fn interned_hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.interned_hash(state);
    }
}

impl FromIterator<Float> for Point {
    fn from_iter<T: IntoIterator<Item = Float>>(iter: T) -> Self {
        Self(iter.into_iter().collect())
    }
}

impl fmt::Display for Point {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "(")?;
        let mut iter = self.0.iter();
        if let Some(first) = iter.next() {
            first.fmt(f)?;
            for elem in iter {
                write!(f, ", ")?;
                elem.fmt(f)?;
            }
        }
        write!(f, ")")?;
        Ok(())
    }
}

impl<V: VectorRef> Add<V> for Point {
    type Output = Point;

    fn add(self, rhs: V) -> Self::Output {
        Point(self.0 + rhs)
    }
}
impl<V: VectorRef> Add<V> for &Point {
    type Output = Point;

    fn add(self, rhs: V) -> Self::Output {
        Point(&self.0 + rhs)
    }
}

impl<V: VectorRef> Sub<V> for Point {
    type Output = Point;

    fn sub(self, rhs: V) -> Self::Output {
        Point(self.0 - rhs)
    }
}
impl<V: VectorRef> Sub<V> for &Point {
    type Output = Point;

    fn sub(self, rhs: V) -> Self::Output {
        Point(&self.0 - rhs)
    }
}

impl Sub<Point> for Point {
    type Output = Vector;

    fn sub(self, rhs: Point) -> Self::Output {
        self.0 - rhs.0
    }
}
impl Sub<Point> for &Point {
    type Output = Vector;

    fn sub(self, rhs: Point) -> Self::Output {
        &self.0 - rhs.0
    }
}
impl Sub<&Point> for Point {
    type Output = Vector;

    fn sub(self, rhs: &Point) -> Self::Output {
        self.0 - &rhs.0
    }
}
impl Sub<&Point> for &Point {
    type Output = Vector;

    fn sub(self, rhs: &Point) -> Self::Output {
        &self.0 - &rhs.0
    }
}

impl AddAssign<Vector> for Point {
    fn add_assign(&mut self, rhs: Vector) {
        self.0 += rhs;
    }
}
impl AddAssign<&Vector> for Point {
    fn add_assign(&mut self, rhs: &Vector) {
        self.0 += rhs;
    }
}
impl SubAssign<Vector> for Point {
    fn sub_assign(&mut self, rhs: Vector) {
        self.0 -= rhs;
    }
}
impl SubAssign<&Vector> for Point {
    fn sub_assign(&mut self, rhs: &Vector) {
        self.0 -= rhs;
    }
}

impl Index<u8> for Point {
    type Output = Float;

    fn index(&self, index: u8) -> &Self::Output {
        &self.0[index]
    }
}
impl IndexMut<u8> for Point {
    fn index_mut(&mut self, index: u8) -> &mut Self::Output {
        &mut self.0[index]
    }
}
