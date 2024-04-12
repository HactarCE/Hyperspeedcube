use hypermath::collections::ApproxHashMap;
use hypermath::prelude::*;
use itertools::Itertools;

use super::{GroupResult, IsometryGroup};

/// Schlafli symbol for a convex polytope.
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
pub struct SchlafliSymbol {
    indices: Vec<usize>,
}
impl SchlafliSymbol {
    /// Constructs an integer Schlafli symbol.
    pub const fn from_indices(indices: Vec<usize>) -> Self {
        Self { indices }
    }

    /// Returns the indices of the Schlafli symbol.
    pub fn indices(&self) -> &[usize] {
        &self.indices
    }

    /// Constructs an integer Schlafli symbol from a string.
    pub fn from_string(string: &str) -> Self {
        let xs = string
            .split(',')
            .map(|s| s.trim().parse().unwrap_or(0))
            .collect_vec();
        Self::from_indices(xs)
    }

    /// Number of dimensions of the polytope described by the Schlafli symbol.
    pub fn ndim(&self) -> u8 {
        self.indices.len() as u8 + 1
    }

    /// Returns the list of mirrors.
    pub fn mirrors(&self) -> Vec<Mirror> {
        let mut ret = vec![];
        let mut last = Vector::unit(0);
        for (i, &index) in self.indices.iter().enumerate() {
            ret.push(Mirror(last.clone()));
            // The final mirror vectors will look like this, with each row as a
            // vector:
            //
            // ```
            // [ ? 0 0 0 0 ]
            // [ ? ? 0 0 0 ]
            // [ 0 ? ? 0 0 ]
            // [ 0 0 ? ? 0 ]
            // [ 0 0 0 ? ? ]
            // ```
            //
            // Each mirror vector is perpendicular to all the others except its
            // neighbors.
            //
            // So to compute each next mirror vector, we only need to consider
            // the previous one. Consider the third mirror vector:
            //
            // ```
            // [ 0 ? ? 0 0 ]
            // ```
            //
            // Only two axes are nonzero, and their values could be anything.
            // The first nonzero axis is irrelevant, because that axis will be
            // zero in the next vector. Let `q` be the value of the second
            // nonzero axis.
            let q = last[i as u8];
            // `dot` is what we want the dot product of the new vector with the
            // previous one to be.
            // This is negated because the mirrors should face each other.
            let dot = -(std::f64::consts::PI as Float / index as Float).cos();
            // Since there's only one axis shared between the last vector and
            // the new one, only that axis will affect the dot product.
            let y = dot / q;
            // Compute the other nonzero axis of the new vector such that the
            // vector will be normalized.
            let z = (1.0 - y * y).sqrt();
            // Actually construct that vector.
            last = Vector::zero(self.ndim());
            last[i as u8] = y;
            last[i as u8 + 1] = z;
        }
        ret.push(Mirror(last));
        ret
    }

    /// Returns a matrix that transforms from the mirror basis (where each
    /// component of the vector gives a distance from a mirror plane) to the
    /// base space.
    pub fn mirror_basis(&self) -> Option<Matrix> {
        Matrix::from_cols(self.mirrors().into_iter().map(|Mirror(m)| m))
            .transpose()
            .inverse()
    }

    /// Returns the list of mirrors as generators.
    pub fn generators(&self) -> Vec<Isometry> {
        self.mirrors().into_iter().map(|m| m.into()).collect()
    }

    /// Constructs the isometry group described by the Schlafli symbol.
    pub fn group(&self) -> GroupResult<IsometryGroup> {
        IsometryGroup::from_generators(&self.generators())
    }

    /// Expands an object by the symmetry.
    pub fn expand<T: Clone + ApproxHashMapKey>(
        &self,
        object: T,
        transform: fn(&Isometry, &T) -> T,
    ) -> Vec<(Isometry, T)> {
        let generators = self.generators();

        let mut seen = ApproxHashMap::new();
        seen.insert(object.clone(), ());

        let mut next_unprocessed_index = 0;
        let mut ret = vec![(Isometry::ident(), object)];
        while next_unprocessed_index < ret.len() {
            let (unprocessed_transform, unprocessed_object) = ret[next_unprocessed_index].clone();
            for gen in &generators {
                let new_object = transform(gen, &unprocessed_object);
                if seen.insert(new_object.clone(), ()).is_none() {
                    ret.push((gen * &unprocessed_transform, new_object));
                }
            }
            next_unprocessed_index += 1;
        }
        ret
    }
}

#[derive(Debug, Clone, PartialEq)]
struct MirrorGenerator {
    mirrors: Vec<Mirror>,
}
impl From<MirrorGenerator> for Isometry {
    fn from(gen: MirrorGenerator) -> Self {
        gen.mirrors
            .into_iter()
            .map(Isometry::from)
            .fold(Isometry::ident(), |a, b| a * b)
    }
}
impl From<MirrorGenerator> for Matrix {
    fn from(gen: MirrorGenerator) -> Self {
        gen.mirrors
            .into_iter()
            .map(Matrix::from)
            .fold(Matrix::EMPTY_IDENT, |a, b| a * b)
    }
}

/// Mirror hyperplane that intersects the origin, defined by its normal vector.
#[derive(Debug, Clone, PartialEq)]
pub struct Mirror(pub Vector);
impl From<Mirror> for Isometry {
    fn from(Mirror(v): Mirror) -> Self {
        Isometry::from_reflection_normalized(v)
    }
}
impl From<Mirror> for Matrix {
    fn from(Mirror(v): Mirror) -> Self {
        let ndim = v.ndim();
        let mut ret = Matrix::ident(ndim);
        for x in 0..ndim {
            for y in 0..ndim {
                *ret.get_mut(x, y) = ret.get(x, y) - 2.0 * v[x] * v[y];
            }
        }
        ret
    }
}

#[cfg(test)]
mod tests {
    use super::super::Group;
    use super::*;

    #[test]
    fn test_cube_group() {
        let g = SchlafliSymbol::from_indices(vec![4, 3]).group().unwrap();

        assert_eq!(48, g.element_count());
    }
}
