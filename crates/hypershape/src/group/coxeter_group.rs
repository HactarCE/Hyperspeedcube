use std::fmt;

use hypermath::collections::approx_hashmap::ApproxHashMapKey;
use hypermath::prelude::*;
use itertools::Itertools;
use smallvec::SmallVec;

use super::{GeneratorId, GroupError, GroupResult, IsometryGroup};

/// Abbreviated generator sequence; generator sequence that may be expressed in
/// terms of another element in the orbit.
#[derive(Debug, Default, Clone)]
pub struct AbbrGenSeq {
    /// Generator indices.
    pub generators: GenSeq,
    /// Index of an optional final element, whose generators should be applied
    /// after `generators`.
    pub end: Option<usize>,
}
impl AbbrGenSeq {
    /// The empty generator sequence, which identifies the initial element in an
    /// orbit.
    pub const INIT: Self = Self {
        generators: GenSeq::INIT,
        end: None,
    };

    /// Constructs a new abbreviated generator sequence that consists of a
    /// sequence of indices followed by the generator sequence of `end`.
    pub fn new(indices: impl IntoIterator<Item = GeneratorId>, end: Option<usize>) -> Self {
        let generators = GenSeq::new(indices);
        AbbrGenSeq { generators, end }
    }
}

/// Generator sequence to reach an element in an orbit.
#[derive(Debug, Default, Clone)]
pub struct GenSeq(pub SmallVec<[GeneratorId; 8]>);
impl GenSeq {
    /// The empty generator sequence, which identifies the initial element in an
    /// orbit.
    pub const INIT: Self = Self(SmallVec::new_const());

    /// Constructs a new generator sequence.
    pub fn new(indices: impl IntoIterator<Item = GeneratorId>) -> Self {
        Self::from_iter(indices)
    }
}
impl FromIterator<GeneratorId> for GenSeq {
    fn from_iter<T: IntoIterator<Item = GeneratorId>>(iter: T) -> Self {
        Self(iter.into_iter().collect())
    }
}

/// Description of a Coxeter group.
#[derive(Debug, Clone)]
pub struct CoxeterGroup {
    /// [Coxeter matrix](https://w.wiki/7SNw), whose adjacent-to-diagonal
    /// entries correspond to the numbers in a linear Schläfli symbol.
    coxeter_matrix: Vec<Vec<usize>>,
    /// Precomputed mirror generators of the group.
    mirrors: Vec<Mirror>,

    /// Matrix that transforms from the mirror basis (where each component of
    /// the vector gives a distance from a mirror plane) to the base space.
    mirror_basis: Matrix,

    /// Minimum number of dimensions in which the Coxeter group is valid.
    ///
    /// This is NOT necessarily the same as the mirror count.
    min_ndim: u8,
}
impl Eq for CoxeterGroup {}
impl PartialEq for CoxeterGroup {
    fn eq(&self, other: &Self) -> bool {
        self.coxeter_matrix == other.coxeter_matrix
    }
}
impl fmt::Display for CoxeterGroup {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(indices) = self.linear_indices() {
            write!(f, "{{{}}}", indices.iter().join(", "))
        } else {
            write!(f, "{:?}", self.coxeter_matrix)
        }
    }
}
impl CoxeterGroup {
    /// Constructs a Coxeter group from a linear Schläfli symbol.
    pub fn new_linear(indices: &[usize], basis: Option<Vec<Vector>>) -> GroupResult<Self> {
        let ndim = check_ndim(indices.len() + 1)?;
        let coxeter_matrix_fn = Self::linear_indices_to_matrix_index_fn(indices);
        Self::from_matrix_index_fn(ndim, coxeter_matrix_fn, basis)
    }
    fn linear_indices_to_matrix_index_fn(indices: &[usize]) -> impl '_ + Fn(u8, u8) -> usize {
        |i, j| {
            if i == j {
                1
            } else if i == j + 1 || j == i + 1 {
                indices[std::cmp::min(i, j) as usize]
            } else {
                2
            }
        }
    }

    /// Constructs and validates a Coxeter group from a function that returns
    /// each element in the Coxeter matrix.
    pub fn from_matrix_index_fn(
        generator_count: u8,
        mut f: impl FnMut(u8, u8) -> usize,
        basis: Option<Vec<Vector>>,
    ) -> GroupResult<Self> {
        Self::from_coxeter_matrix(
            (0..generator_count)
                .map(|i| (0..generator_count).map(|j| f(i, j)).collect())
                .collect(),
            basis,
        )
    }

    /// Constructs and validates a Coxeter group from a Coxeter matrix.
    pub fn from_coxeter_matrix(
        coxeter_matrix: Vec<Vec<usize>>,
        basis: Option<Vec<Vector>>,
    ) -> GroupResult<Self> {
        let mirror_count = check_ndim(coxeter_matrix.len())?;

        // Validate basis.
        let basis_matrix = match basis {
            Some(basis) => {
                let basis_ndim = basis.iter().map(|v| v.ndim()).max().unwrap_or(mirror_count);
                if basis.len() != mirror_count as usize {
                    return Err(GroupError::BadBasis);
                }
                let Some(basis_vectors) = basis
                    .into_iter()
                    .map(|v| v.normalize())
                    .collect::<Option<Vec<_>>>()
                else {
                    return Err(GroupError::BadBasis);
                };
                for (i, v1) in basis_vectors.iter().enumerate() {
                    for v2 in &basis_vectors[..i] {
                        if approx_eq(&v1.dot(v2).abs(), &1.0) {
                            return Err(GroupError::BadBasis);
                        }
                    }
                }
                Some(Matrix::from_cols(basis_vectors.iter().pad_using(
                    basis_ndim as usize,
                    |_| const { &Vector::EMPTY },
                )))
            }
            None => None,
        };

        let min_ndim = match &basis_matrix {
            Some(mat) => mat.ndim(),
            None => mirror_count,
        };

        // Validate indices.
        {
            if coxeter_matrix
                .iter()
                .any(|r| r.len() != coxeter_matrix.len())
            {
                // Index matrix is not square
                return Err(GroupError::BadCD);
            }

            for i in 0..mirror_count as _ {
                for j in 0..=i {
                    if coxeter_matrix[i][j] < 1 {
                        // Index matrix has non-positive integer
                        return Err(GroupError::BadCD);
                    }
                    if (i == j) != (coxeter_matrix[i][j] == 1) {
                        // Index matrix has ones off diagonal or no ones on diagonal
                        return Err(GroupError::BadCD);
                    }
                    if coxeter_matrix[i][j] != coxeter_matrix[j][i] {
                        // Index matrix is not symmetric
                        return Err(GroupError::BadCD);
                    }
                }
            }
        }

        // Compute mirrors.
        let mut mirrors = vec![];
        // The final mirror vectors will look like this, with each row as a
        // vector:
        //
        // ```
        // [ ? 0 0 0 0 ]
        // [ ? ? 0 0 0 ]
        // [ ? ? ? 0 0 ]
        // [ ? ? ? ? 0 ]
        // [ ? ? ? ? ? ]
        // ```
        //
        // If this matrix is `L`, `L Lᵀ = A`, where `A` is the Schläfli
        // matrix of the Coxeter-Dynkin diagram. This is a Cholesky
        // decomposition. We use the Cholesky–Banachiewicz algorithm.
        // https://en.wikipedia.org/wiki/Cholesky_decomposition#Computation
        for i in 0..coxeter_matrix.len() {
            mirrors.push(Vector::zero(i as u8 + 1));
            for j in 0..=i {
                let mut sum = 0.0;
                for k in 0..j {
                    sum += mirrors[i][k as u8] * mirrors[j][k as u8];
                }

                let mirror_dot =
                    -(std::f64::consts::PI as Float / coxeter_matrix[i][j] as Float).cos();
                if i == j {
                    let val = mirror_dot - sum;
                    if val < 0.0 {
                        return Err(GroupError::HyperbolicCD);
                    }
                    mirrors[i][j as u8] = val.sqrt();
                } else {
                    mirrors[i][j as u8] = 1.0 / mirrors[j][j as u8] * (mirror_dot - sum);
                }
            }
        }

        // Compute mirror basis.
        let mirror_basis = Matrix::from_cols(&mirrors)
            .transpose()
            .inverse()
            .ok_or(GroupError::EuclideanCD)?;

        // Transform mirrors and mirror basis matrix.
        let mirrors = mirrors
            .into_iter()
            .map(|v| {
                Mirror(match &basis_matrix {
                    Some(mat) => mat * v,
                    None => v,
                })
            })
            .collect();
        let mirror_basis = match &basis_matrix {
            Some(mat) => mat * mirror_basis,
            None => mirror_basis,
        };

        Ok(Self {
            coxeter_matrix,
            mirrors,

            mirror_basis,

            min_ndim,
        })
    }

    /// Returns the Coxeter matrix.
    pub fn coxeter_matrix(&self) -> &[Vec<usize>] {
        &self.coxeter_matrix
    }
    /// Returns the indices of the linear Schläfli symbol, or `None` if this
    /// group cannot be written as one.
    pub fn linear_indices(&self) -> Option<Vec<usize>> {
        let indices = (1..self.mirror_count())
            .map(|i| self.coxeter_matrix[i as usize][i as usize - 1])
            .collect_vec();
        let f = Self::linear_indices_to_matrix_index_fn(&indices);
        for i in 0..self.mirror_count() {
            for j in 0..self.mirror_count() {
                if self.coxeter_matrix[i as usize][j as usize] != f(i, j) {
                    return None;
                }
            }
        }
        drop(f);
        Some(indices)
    }

    /// Returns the number of mirror generators for the Coxeter group.
    pub fn mirror_count(&self) -> u8 {
        self.coxeter_matrix.len() as u8
    }
    /// Minimum number of dimensions required for the Coxeter group. This may be
    /// more than the number of mirrors, if the group has been transformed.
    ///
    /// At time of writing, transforming groups has not yet been implemented, so
    /// this method is the same as `mirror_count()`.
    pub fn min_ndim(&self) -> u8 {
        self.min_ndim
    }

    /// Returns the list of mirrors.
    pub fn mirrors(&self) -> &[Mirror] {
        &self.mirrors
    }

    /// Returns a matrix that transforms from the mirror basis (where each
    /// component of the vector gives a distance from a mirror plane) to the
    /// base space.
    pub fn mirror_basis(&self) -> &Matrix {
        &self.mirror_basis
    }

    /// Returns the list of mirrors as generators.
    pub fn generators(&self) -> Vec<pga::Motor> {
        self.mirrors.iter().map(Mirror::motor).collect()
    }

    /// Constructs the full Coxeter group from its description.
    pub fn group(&self) -> GroupResult<IsometryGroup> {
        IsometryGroup::from_generators(&self.generators())
    }

    /// Returns the orbit of an object under the symmetry.
    pub fn orbit<T: ApproxHashMapKey + Clone + TransformByMotor>(
        &self,
        object: T,
        chiral: bool,
    ) -> Vec<(AbbrGenSeq, pga::Motor, T)> {
        let generators = self
            .generators()
            .into_iter()
            .enumerate()
            .map(|(i, g)| (GeneratorId(i as u8), g));

        let generators = if chiral {
            itertools::iproduct!(generators.clone(), generators)
                .map(|((i, g1), (j, g2))| (GenSeq::new([i, j]), g1 * g2))
                .collect_vec()
        } else {
            generators.map(|(i, g)| (GenSeq::new([i]), g)).collect_vec()
        };

        super::orbit(&generators, object)
    }
}
impl TransformByMotor for CoxeterGroup {
    fn transform_by(&self, m: &pga::Motor) -> Self {
        Self {
            coxeter_matrix: self.coxeter_matrix.clone(),
            mirrors: self
                .mirrors
                .iter()
                .map(|mirror| m.transform(mirror))
                .collect(),
            // TODO: `impl Mul<Matrix> for pga::Motor` or similar
            mirror_basis: Matrix::from_cols(
                self.mirror_basis.cols().map(|v| m.transform_vector(v)),
            ),
            min_ndim: std::cmp::max(self.min_ndim, m.ndim()),
        }
    }
}

/// Mirror hyperplane that intersects the origin, defined by its normal vector.
#[derive(Debug, Clone, PartialEq)]
pub struct Mirror(Vector);
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
impl Mirror {
    /// Returns the normal vector of the mirror.
    pub fn normal(&self) -> &Vector {
        &self.0
    }
    /// Returns the hyperplane of the mirror, or `None` if the mirror is
    /// degenerate. The hyperplane may be flipped and still correspond to the
    /// same mirror.
    pub fn hyperplane(&self) -> Option<Hyperplane> {
        Hyperplane::new(&self.0, 0.0)
    }
    /// Returns the motor representing the mirror transformation.
    pub fn motor(&self) -> pga::Motor {
        pga::Motor::normalized_vector_reflection(&self.0)
    }
}
impl TransformByMotor for Mirror {
    fn transform_by(&self, m: &pga::Motor) -> Self {
        Self(m.transform_vector(&self.0))
    }
}

fn check_ndim(ndim: impl TryInto<u8>) -> GroupResult<u8> {
    ndim.try_into().map_err(|_| GroupError::TooHighDimensional)
}

#[cfg(test)]
mod tests {
    use super::super::Group;
    use super::*;

    #[test]
    fn test_cube_group() {
        let g = CoxeterGroup::new_linear(&[4, 3], None)
            .unwrap()
            .group()
            .unwrap();

        assert_eq!(48, g.element_count());
    }
}
