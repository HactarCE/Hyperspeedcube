use std::ops::Mul;

use super::{Matrix, Vector, VectorRef};

/// N-1 dimensional hyperplane, by normal and signed distance from the origin
#[derive(Debug, Clone, PartialEq)]
pub struct Hyperplane {
    /// Must be normalised.
    pub normal: Vector,
    /// Distance from the origin signed by the normal direction
    pub distance: f32,
}
impl Hyperplane {
    pub fn from_pole(pole: impl VectorRef) -> Option<Self> {
        Some(Self {
            normal: pole.normalize()?,
            distance: pole.mag(),
        })
    }

    /// Returns the position of the point on the hyperplane nearest the origin.
    pub fn pole(&self) -> Vector {
        &self.normal * self.distance
    }

    /// Returns the shortest distance to a point from the plane, signed by the
    /// normal direction.
    pub fn distance_to(&self, point: impl VectorRef) -> f32 {
        // -(self.pole() - point).dot(&self.normal)
        point.dot(&self.normal) - self.distance
    }
}
impl approx::AbsDiffEq for Hyperplane {
    type Epsilon = f32;

    fn default_epsilon() -> Self::Epsilon {
        super::EPSILON
    }

    fn abs_diff_eq(&self, other: &Self, epsilon: Self::Epsilon) -> bool {
        self.normal.abs_diff_eq(&other.normal, epsilon)
            && self.distance.abs_diff_eq(&other.distance, epsilon)
    }
}
impl<'a> Mul<&'a Hyperplane> for &'a Matrix {
    type Output = Hyperplane;

    fn mul(self, rhs: &'a Hyperplane) -> Self::Output {
        let normal = self * &rhs.normal;
        let mag = normal.mag();
        Hyperplane {
            normal: normal / mag,
            distance: rhs.distance * mag,
        }
    }
}
impl_forward_bin_ops_to_ref! {
    impl Mul<Hyperplane> for Matrix { fn mul() -> Hyperplane }
}
