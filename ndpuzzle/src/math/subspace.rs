use super::*;

/// Subspace at the intersection of a set of hyperplanes that all pass through
/// the origin.
pub struct SubspaceMeet {
    /// Dual of the intersection.
    dual: Multivector,
}
impl SubspaceMeet {
    /// Constructs a subspace meet from a set of hyperplane normals.
    pub fn from_normals<V: VectorRef>(normals: impl IntoIterator<Item = V>) -> Self {
        // The intersection ("meet") is the dual of the exterior product. This
        // dual is much easier to work with in this case.

        let dual = normals
            .into_iter()
            .fold(Multivector::scalar(1.0), |m, normal| {
                // Compute the exterior product.
                let new_result = &m ^ &Multivector::from(normal);
                // If the exterior product is zero, then the new normal is
                // parallel to `m` so we don't need it.
                if new_result.is_approx_zero() {
                    m
                } else {
                    new_result
                }
            })
            .normalize()
            .unwrap_or(Multivector::scalar(1.0));

        Self { dual }
    }

    /// Projects a vector onto the subspace.
    pub fn project_vector(&self, vector: impl VectorRef) -> Vector {
        let ret = ((&self.dual ^ &Multivector::from(&vector)) * self.dual.conjugate())
            .grade_project_to_vector();
        dbg!(ret.mag(), vector.mag());
        ret
    }
}
