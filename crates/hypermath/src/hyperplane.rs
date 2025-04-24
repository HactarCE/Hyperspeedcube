//! Hyperplanes in Euclidean space.

use std::fmt;

use crate::collections::approx_hashmap::{FloatHash, VectorHash};
use crate::{
    AXIS_NAMES, ApproxHashMapKey, Float, Point, PointWhichSide, Vector, VectorRef, approx_cmp,
    is_approx_nonzero,
};

/// Hyperplane in Euclidean space, which is also used to represent a half-space.
#[derive(Debug, Clone, PartialEq)]
pub struct Hyperplane {
    /// Normalized normal vector.
    pub(crate) normal: Vector,
    /// Distance from the plane to the origin, perpendicular to the normal
    /// vector.
    pub(crate) distance: Float,
}

impl fmt::Display for Hyperplane {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut terms = self
            .normal
            .iter_nonzero()
            .map(|(i, x)| {
                let i = i as usize;
                (&AXIS_NAMES[i..i + 1], x)
            })
            .chain(is_approx_nonzero(&self.distance).then_some(("", self.distance)));
        if let Some((axis, coef)) = terms.next() {
            write!(f, "{coef}{axis}")?;
        }
        for (axis, coef) in terms {
            write!(f, " + {coef}{axis}")?;
        }
        write!(f, " = 0")?;

        Ok(())
    }
}

impl approx::AbsDiffEq for Hyperplane {
    type Epsilon = Float;

    fn default_epsilon() -> Self::Epsilon {
        crate::EPSILON
    }

    fn abs_diff_eq(&self, other: &Self, epsilon: Self::Epsilon) -> bool {
        self.normal.approx_eq(&other.normal, epsilon)
            && (self.distance - other.distance).abs() <= epsilon
    }
}

impl Hyperplane {
    /// Constructs a new hyperplane from a normal vector and a distance. Returns
    /// `None` if `normal` is approximately zero.
    ///
    /// The normal vector need not be normalized.
    pub fn new(normal: impl VectorRef, distance: Float) -> Option<Self> {
        let normal = normal.normalize()?;
        Some(Self { normal, distance })
    }
    /// Constructs a new hyperplane from a pole vector. Returns `None` if `pole`
    /// is approximately zero.
    pub fn from_pole(pole: impl VectorRef) -> Option<Self> {
        let mag = pole.mag();
        Self::new(pole, mag)
    }
    /// Constructs a new hyperplane from a normal vector and a point that it
    /// passes through. Returns `None` if `normal` is approximately zero.
    pub fn through_point(normal: impl VectorRef, point: impl VectorRef) -> Option<Self> {
        let normal = normal.normalize()?;
        let distance = normal.dot(point);
        Some(Self { normal, distance })
    }

    /// Returns the (normalized) normal vector of the hyperplane.
    pub fn normal(&self) -> &Vector {
        &self.normal
    }
    /// Returns the distance from the plane to the origin, perpendicular to the
    /// normal vector.
    pub fn distance(&self) -> Float {
        self.distance
    }
    /// Returns the pole of the hyperplane, which may be zero.
    pub fn pole(&self) -> Vector {
        &self.normal * self.distance
    }

    /// Returns the signed perpendicular distance of a point from the plane.
    pub fn signed_distance_to_point(&self, p: &Point) -> Float {
        self.normal.dot(p.as_vector()) - self.distance
    }

    /// Returns a hyperplane in the same location but with the opposite
    /// orientation.
    #[must_use]
    pub fn flip(&self) -> Self {
        Self {
            normal: -&self.normal,
            distance: -self.distance,
        }
    }

    /// Returns the location of a point based on its height above or below the
    /// plane.
    fn location_of_point_from_signed_distance(h: Float) -> PointWhichSide {
        match approx_cmp(&h, &0.0) {
            std::cmp::Ordering::Less => PointWhichSide::Inside,
            std::cmp::Ordering::Equal => PointWhichSide::On,
            std::cmp::Ordering::Greater => PointWhichSide::Outside,
        }
    }

    /// Returns the location of a point relative to the hyperplane. For positive
    /// distance, the inside of the hyperplane contains the origin.
    pub fn location_of_point(&self, p: &Point) -> PointWhichSide {
        Self::location_of_point_from_signed_distance(self.signed_distance_to_point(p))
    }

    /// Returns the intersection of the hyperplane with a line segment.
    pub fn intersection_with_line_segment(
        &self,
        [a, b]: [&Point; 2],
    ) -> HyperplaneLineIntersection {
        let ha = self.signed_distance_to_point(a);
        let hb = self.signed_distance_to_point(b);
        let a_loc = Self::location_of_point_from_signed_distance(ha);
        let b_loc = Self::location_of_point_from_signed_distance(hb);
        let intersection = (a_loc != b_loc).then(|| Point::normalized_weighted_sum(a, hb, b, -ha));
        HyperplaneLineIntersection {
            a_loc,
            b_loc,
            intersection,
        }
    }
}

/// Intersection of a hyperplane and a line segment.
pub struct HyperplaneLineIntersection {
    /// Which side of the hyperplane contains the first point.
    pub a_loc: PointWhichSide,
    /// Which side of the hyperplane contains the second point.
    pub b_loc: PointWhichSide,
    /// Intersection point of the line segment and hyperplane, if the line
    /// segment touches the hyperplane.
    pub intersection: Option<Point>,
}

impl ApproxHashMapKey for Hyperplane {
    type Hash = VectorHash;

    fn approx_hash(&self, float_hash_fn: impl FnMut(Float) -> FloatHash) -> Self::Hash {
        let mut v = self.normal.clone();
        v.0.insert(0, self.distance);
        v.approx_hash(float_hash_fn)
    }
}
