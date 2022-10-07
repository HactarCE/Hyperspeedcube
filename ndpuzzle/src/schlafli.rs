use itertools::Itertools;

use crate::math::*;

/// Schlafli symbol for a convex polytope.
pub struct SchlafliSymbol {
    indices: Vec<usize>,
}
impl SchlafliSymbol {
    /// Constructs an integer Schlafli symbol.
    pub fn from_indices(indices: Vec<usize>) -> Self {
        Self { indices }
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
            let dot = (std::f32::consts::PI / index as f32).cos();
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

    /// Returns the list of mirrors as generators.
    pub fn generators(self) -> Vec<Rotoreflector> {
        self.mirrors().into_iter().map(|m| m.into()).collect()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MirrorGenerator {
    mirrors: Vec<Mirror>,
}
impl From<MirrorGenerator> for Rotoreflector {
    fn from(gen: MirrorGenerator) -> Self {
        gen.mirrors
            .into_iter()
            .map(|Mirror(v)| Rotoreflector::from_reflection(v))
            .fold(Rotoreflector::ident(), |a, b| a * b)
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

#[derive(Debug, Clone, PartialEq)]
pub struct Mirror(pub Vector);
impl From<Mirror> for Rotoreflector {
    fn from(Mirror(v): Mirror) -> Self {
        Rotoreflector::from_reflection(v)
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
