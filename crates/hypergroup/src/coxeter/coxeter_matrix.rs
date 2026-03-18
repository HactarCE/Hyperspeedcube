use std::{fmt, sync::Arc};

use hypermath::prelude::*;
use itertools::Itertools;

use crate::{
    AbstractGroupLut, FactorGroupIsometries, Group, GroupError, GroupResult, IsometryGroup,
    PerGenerator,
};

/// [Coxeter matrix](https://w.wiki/7SNw).
///
/// All entries along the diagonal are 1. Entries adjacent to the diagonal
/// correspond to the numbers in a linear Schläfli symbol.
///
/// TODO: consider merging with `Coxeter`
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CoxeterMatrix {
    entries: Vec<Vec<u16>>,
}

impl fmt::Display for CoxeterMatrix {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(indices) = self.linear_indices() {
            write!(f, "{indices:?}")
        } else {
            write!(f, "{:?}", self.entries)
        }
    }
}

impl CoxeterMatrix {
    /// Constructs a Coxeter group from a linear Schläfli symbol.
    pub fn new_linear(indices: &[u16]) -> GroupResult<Self> {
        let ndim = check_ndim(indices.len() + 1)?;
        let coxeter_matrix_fn = Self::linear_indices_to_matrix_index_fn(indices);
        Self::from_matrix_index_fn(ndim, coxeter_matrix_fn)
    }

    fn linear_indices_to_matrix_index_fn(indices: &[u16]) -> impl '_ + Fn(u8, u8) -> u16 {
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

    /// Returns the indices of the linear Schläfli symbol, or `None` if this
    /// group cannot be written as as a linear Schläfli symbolr.
    pub fn linear_indices(&self) -> Option<Vec<u16>> {
        let indices = (1..self.mirror_count())
            .map(|i| self.entries[i][i - 1])
            .collect_vec();
        let f = Self::linear_indices_to_matrix_index_fn(&indices);
        for i in 0..self.mirror_count() {
            for j in 0..self.mirror_count() {
                if self.entries[i][j] != f(i as u8, j as u8) {
                    return None;
                }
            }
        }
        drop(f);
        Some(indices)
    }

    /// Constructs and validates a Coxeter group from a function that returns
    /// each element in the Coxeter matrix.
    pub fn from_matrix_index_fn(
        generator_count: u8,
        mut f: impl FnMut(u8, u8) -> u16,
    ) -> GroupResult<Self> {
        Self::from_coxeter_matrix(
            (0..generator_count)
                .map(|i| (0..generator_count).map(|j| f(i, j)).collect())
                .collect(),
        )
    }

    /// Constructs and validates a Coxeter group from a Coxeter matrix.
    pub fn from_coxeter_matrix(coxeter_matrix: Vec<Vec<u16>>) -> GroupResult<Self> {
        let mirror_count = check_ndim(coxeter_matrix.len())?;

        // Validate indices.
        {
            if coxeter_matrix
                .iter()
                .any(|r| r.len() != coxeter_matrix.len())
            {
                // Index matrix is not square
                return Err(GroupError::BadCD);
            }

            #[expect(clippy::needless_range_loop)] // it's clearer this way
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

        Ok(Self {
            entries: coxeter_matrix,
        })
    }

    pub fn entries(&self) -> &Vec<Vec<u16>> {
        &self.entries
    }

    pub fn mirror_count(&self) -> usize {
        self.entries.len()
    }

    /// Returns the mirror vector generators for a finite (spherical) Coxeter
    /// group.
    ///
    /// Returns an error if the Coxeter group is infinite (Euclidean or
    /// hyperbolic).
    pub fn spherical_mirrors(&self) -> GroupResult<SphericalCoxeterMirrors> {
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
        // If this matrix is `L`, `L Lᵀ = A`, where `A` is the Schläfli matrix
        // of the Coxeter-Dynkin diagram. This is a Cholesky decomposition. We
        // use the Cholesky–Banachiewicz algorithm.
        // https://en.wikipedia.org/wiki/Cholesky_decomposition#Computation
        let mut vectors = vec![];
        for i in 0..self.mirror_count() {
            vectors.push(Vector::zero(i as u8 + 1));
            for j in 0..=i {
                let mut sum = 0.0;
                for k in 0..j {
                    sum += vectors[i][k as u8] * vectors[j][k as u8];
                }

                let mirror_dot =
                    -(std::f64::consts::PI as Float / self.entries[i][j] as Float).cos();
                if i == j {
                    let val = mirror_dot - sum;
                    if val < 0.0 {
                        return Err(GroupError::HyperbolicCD);
                    }
                    vectors[i][j as u8] = val.sqrt();
                } else {
                    vectors[i][j as u8] = 1.0 / vectors[j][j as u8] * (mirror_dot - sum);
                }
            }
        }

        // Compute mirror basis.
        let basis = Matrix::from_cols(&vectors)
            .transpose()
            .inverse()
            .ok_or(GroupError::EuclideanCD)?;

        Ok(SphericalCoxeterMirrors { vectors, basis })
    }

    pub fn spherical_mirror_generators(&self) -> GroupResult<PerGenerator<pga::Motor>> {
        self.spherical_mirrors()?
            .vectors
            .iter()
            .map(pga::Motor::vector_reflection)
            .collect::<Option<PerGenerator<pga::Motor>>>()
            .ok_or(GroupError::BadCD)
    }
    pub fn spherical_chiral_generators(&self) -> GroupResult<PerGenerator<pga::Motor>> {
        let mut mirror_generators = self.spherical_mirror_generators()?.into_values();
        let Some(first) = mirror_generators.next() else {
            return Ok(PerGenerator::new());
        };
        Ok(mirror_generators.map(|g| g * &first).collect())
    }

    fn abstract_group_lut(&self) -> GroupResult<AbstractGroupLut> {
        super::todd_coxeter::construct_group(self.to_string(), self)
    }

    /// Constructs the full group structure.
    pub fn group(&self) -> GroupResult<Group> {
        self.abstract_group_lut()?.try_into()
    }

    /// Constructs the full group structure.
    pub fn isometry_group(&self) -> GroupResult<IsometryGroup> {
        let group = self.abstract_group_lut()?;
        let mirror_generators = self.spherical_mirror_generators()?;
        let isometries =
            FactorGroupIsometries::from_generators_unchecked(&group, &mirror_generators);
        IsometryGroup::from_factors([(Arc::new(group), Arc::new(isometries))])
    }

    pub fn chiral_isometry_group(&self) -> GroupResult<IsometryGroup> {
        IsometryGroup::from_generators(
            format!("chiral {self}"),
            self.spherical_chiral_generators()?,
        )
    }
}

/// Mirrors for a finite (spherical) Coxeter group.
pub struct SphericalCoxeterMirrors {
    /// Reflection vectors, perpendicular to their mirror (hyper)planes.
    pub vectors: Vec<Vector>,
    /// Basis for [Dynkin notation].
    ///
    /// [Dynkin notation]:
    ///     https://bendwavy.org/klitzing/explain/dynkin-notation.htm
    pub basis: Matrix,
}

fn check_ndim(ndim: impl TryInto<u8>) -> GroupResult<u8> {
    ndim.try_into().map_err(|_| GroupError::TooHighDimensional)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cube_group() {
        let g = CoxeterMatrix::new_linear(&[4, 3]).unwrap().group().unwrap();

        assert_eq!(48, g.element_count());
    }
}
