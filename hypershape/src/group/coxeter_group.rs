use core::fmt;

use hypermath::collections::approx_hashmap::ApproxHashMapKey;
use hypermath::collections::ApproxHashMap;
use hypermath::prelude::*;
use itertools::Itertools;

use super::{FiniteCoxeterGroup, GroupError, GroupResult, IsometryGroup};

/// Description of a Coxeter group.
#[derive(Debug, Default, Clone)]
pub struct CoxeterGroup {
    /// [Coxeter matrix](https://w.wiki/7SNw), whose adjacent-to-diagonal
    /// entries correspond to the numbers in a linear Schläfli symbol.
    coxeter_matrix: Vec<Vec<usize>>,
    /// Precomputed mirror generators of the group.
    mirrors: Vec<Mirror>,

    /// Minimum number of dimensions in which the Coxeter group is valid.
    ndim: u8,
}
impl Eq for CoxeterGroup {}
impl PartialEq for CoxeterGroup {
    fn eq(&self, other: &Self) -> bool {
        self.coxeter_matrix == other.coxeter_matrix
    }
}
impl TryFrom<FiniteCoxeterGroup> for CoxeterGroup {
    type Error = GroupError;

    fn try_from(value: FiniteCoxeterGroup) -> Result<Self, Self::Error> {
        Self::from_matrix_index_fn(value.generator_count(), |i, j| {
            value.coxeter_matrix_element(i, j) as _
        })
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
    pub fn new_linear(indices: &[usize]) -> GroupResult<Self> {
        let ndim = check_ndim(indices.len() + 1)?;

        Self::from_matrix_index_fn(ndim, Self::linear_indices_to_matrix_index_fn(indices))
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
    ) -> GroupResult<Self> {
        Self::from_coxeter_matrix(
            (0..generator_count)
                .map(|i| (0..generator_count).map(|j| f(i, j)).collect())
                .collect(),
        )
    }

    /// Constructs and validates a Coxeter group from a Coxeter matrix.
    pub fn from_coxeter_matrix(coxeter_matrix: Vec<Vec<usize>>) -> GroupResult<Self> {
        let ndim = check_ndim(coxeter_matrix.len())?;

        // Validate indices.
        {
            if coxeter_matrix
                .iter()
                .any(|r| r.len() != coxeter_matrix.len())
            {
                // Index matrix is not square
                return Err(GroupError::BadCD);
            }

            for i in 0..ndim as _ {
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
            mirrors.push(Mirror(Vector::zero(i as u8 + 1)));
            for j in 0..=i {
                let mut sum = 0.0;
                for k in 0..j {
                    sum += mirrors[i].0[k as u8] * mirrors[j].0[k as u8];
                }

                let mirror_dot =
                    -(std::f64::consts::PI as Float / coxeter_matrix[i][j] as Float).cos();
                if i == j {
                    let val = mirror_dot - sum;
                    if val < 0.0 {
                        return Err(GroupError::HyperbolicCD);
                    }
                    mirrors[i].0[j as u8] = val.sqrt();
                } else {
                    mirrors[i].0[j as u8] = 1.0 / mirrors[j].0[j as u8] * (mirror_dot - sum);
                }
            }
        }

        Ok(Self {
            coxeter_matrix,
            mirrors,

            ndim,
        })
    }

    /// Returns the Coxeter matrix.
    pub fn coxeter_matrix(&self) -> &[Vec<usize>] {
        &self.coxeter_matrix
    }
    /// Returns the indices of the linear Schläfli symbol, or `None` if this
    /// group cannot be written as one.
    pub fn linear_indices(&self) -> Option<Vec<usize>> {
        let indices = (1..self.ndim)
            .map(|i| self.coxeter_matrix[i as usize][i as usize - 1])
            .collect_vec();
        let f = Self::linear_indices_to_matrix_index_fn(&indices);
        for i in 0..self.ndim {
            for j in 0..self.ndim {
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
        self.mirror_count()
    }

    /// Returns the list of mirrors.
    pub fn mirrors(&self) -> &[Mirror] {
        &self.mirrors
    }

    /// Returns a matrix that transforms from the mirror basis (where each
    /// component of the vector gives a distance from a mirror plane) to the
    /// base space.
    pub fn mirror_basis(&self) -> GroupResult<Matrix> {
        let cols = self.mirrors.iter().map(|Mirror(v)| v.clone());
        Matrix::from_cols(cols.pad_using(self.ndim as _, |_| Vector::EMPTY))
            .transpose()
            .inverse()
            .ok_or(GroupError::EuclideanCD)
    }

    /// Returns the list of mirrors as generators.
    pub fn generators(&self) -> Vec<pga::Motor> {
        self.mirrors.iter().map(|m| m.motor(self.ndim)).collect()
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
    ) -> Vec<(pga::Motor, T)> {
        let mut generators = self.generators();
        if chiral {
            generators = itertools::iproduct!(&generators, &generators)
                .map(|(g1, g2)| g1 * g2)
                .collect();
        }

        let mut seen = ApproxHashMap::new();
        seen.insert(object.clone(), ());

        let mut next_unprocessed_index = 0;
        let mut ret = vec![(pga::Motor::ident(self.min_ndim()), object)];
        while next_unprocessed_index < ret.len() {
            let (unprocessed_transform, unprocessed_object) = ret[next_unprocessed_index].clone();
            for gen in &generators {
                let new_object = gen.transform(&unprocessed_object);
                if seen.insert(new_object.clone(), ()).is_none() {
                    ret.push((gen * &unprocessed_transform, new_object));
                }
            }
            next_unprocessed_index += 1;
        }
        ret
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
    /// Returns the hyperplane of the mirror, or `None` if the mirror is
    /// degenerate. The hyperplane may be flipped and still correspond to the
    /// same mirror.
    pub fn hyperplane(&self) -> Option<Hyperplane> {
        Hyperplane::new(&self.0, 0.0)
    }
    /// Returns the motor representing the mirror transformation.
    pub fn motor(&self, ndim: u8) -> pga::Motor {
        pga::Motor::normalized_vector_reflection(ndim, &self.0)
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
        let g = CoxeterGroup::new_linear(&[4, 3]).unwrap().group().unwrap();

        assert_eq!(48, g.element_count());
    }
}
