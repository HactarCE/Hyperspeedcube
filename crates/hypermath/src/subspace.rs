//! Linear subspace type.

use approx_collections::Precision;

use crate::{APPROX, Vector, VectorRef};

/// Linear subspace computed using the Gram-Schmidt process.
#[derive(Debug, Default, Clone)]
pub struct Subspace {
    /// Orthonormal basis for the subspace.
    basis: Vec<Vector>,
}

impl Subspace {
    /// Returns the 0-dimensional subspace, which contains only the origin.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns an orthonormal basis for the subspace.
    pub fn basis(&self) -> &[Vector] {
        &self.basis
    }

    /// Returns the number of dimensions of the subspace, which is equivalent to
    /// the number of basis vectors.
    pub fn ndim(&self) -> u8 {
        self.basis.len() as u8
    }

    /// Projects a vector onto the subspace.
    pub fn project(&self, v: impl VectorRef) -> Vector {
        self.basis.iter().filter_map(|u| v.projected_to(u)).sum()
    }

    /// Returns whether the subspace contains a vector.
    pub fn contains(&self, v: &Vector) -> bool {
        self.contains_with_prec(v, APPROX)
    }

    /// Same as `Self::contains()`, but with a custom precision.
    pub fn contains_with_prec(&self, v: &Vector, prec: Precision) -> bool {
        prec.eq(&self.project(v), v)
    }

    /// Adds a vector to the subspace. Returns `true` if the dimensionality of
    /// the subspace increased by 1, or `false` if it remained the same.
    pub fn add(&mut self, v: &Vector) -> bool {
        self.add_with_prec(v, APPROX)
    }

    /// Same as [`Self::add()`], but with a custom precision.
    pub fn add_with_prec(&mut self, v: &Vector, prec: Precision) -> bool {
        let projection = self.project(v);
        if prec.ne(v, &projection)
            && let Some(new_basis_vector) = (v - projection).normalize()
        {
            self.basis.push(new_basis_vector);
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_subspace() {
        let mut subspace = Subspace::new();
        assert_eq!(subspace.ndim(), 0);
        assert!(!subspace.add(&vector![]));
        assert_eq!(subspace.ndim(), 0);

        assert!(subspace.add(&vector![1.0, 1.0]));
        assert_eq!(subspace.ndim(), 1);
        assert!(!subspace.add(&vector![-2.0, -2.0]));
        assert_eq!(subspace.ndim(), 1);

        assert!(subspace.add(&vector![2.0, 1.0]));
        assert_eq!(subspace.ndim(), 2);
        assert!(!subspace.add(&vector![0.0, 3.0]));
        assert_eq!(subspace.ndim(), 2);

        assert!(!subspace.add(&vector![]));
        assert_eq!(subspace.ndim(), 2);

        assert!(subspace.add(&vector![1.0, 1.0, -1.0]));
        assert_eq!(subspace.ndim(), 3);

        for u in subspace.basis() {
            for v in subspace.basis() {
                if u != v {
                    assert!(APPROX.eq_zero(u.dot(v)));
                }
            }
        }
    }
}
