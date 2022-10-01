use super::{Vector, VectorRef};

/// N-1 dimensional hyperplane, by normal and signed distance from the origin
#[derive(Debug, Clone, PartialEq)]
pub struct Hyperplane {
    /// Must be normalised.
    pub normal: Vector<f32>,
    /// Distance from the origin signed by the normal direction
    pub distance: f32,
}
impl Hyperplane {
    /// Returns the position of the point on the hyperplane nearest the origin.
    pub fn pole(&self) -> Vector<f32> {
        &self.normal * self.distance
    }

    /// Returns the shortest distance to a point from the plane, signed by the normal direction
    pub fn distance_to(&self, point: impl VectorRef<f32>) -> f32 {
        // -(self.pole() - point).dot(&self.normal)
        point.dot(&self.normal) - self.distance
    }

    /// Returns whether this hyperplane is approximately equal to another
    pub fn approx_eq(&self, other: &Hyperplane, epsilon: f32) -> bool {
        ((self.distance - other.distance).abs() < epsilon)
            && self.normal.approx_eq(&other.normal, epsilon)
    }
}
