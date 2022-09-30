//! N-dimensional matrix math.

use num_traits::{Num, Signed};
use std::ops::*;

use super::permutations;
use super::vector::{Vector, VectorRef};

/// N-by-N square matrix. Indexing out of bounds returns the corresponding
/// element from the infinite identity matrix.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Matrix<N: Clone + Num> {
    /// Number of dimensions of the matrix.
    ndim: u8,
    /// Elements stored in **column-major** order.
    elems: Vec<N>,
}
impl<N: Clone + Num> Matrix<N> {
    /// 0-by-0 matrix that functions as the identity matrix.
    pub const EMPTY_IDENT: Self = Matrix {
        ndim: 0,
        elems: vec![],
    };

    /// Constructs a matrix with all zeros.
    pub fn zero(ndim: u8) -> Self {
        Self {
            ndim,
            elems: vec![N::zero(); ndim as usize * ndim as usize],
        }
    }
    /// Constructs an identity matrix.
    pub fn ident(ndim: u8) -> Self {
        let mut ret = Self::zero(ndim);
        for i in 0..ndim {
            *ret.get_mut(i, i) = N::one();
        }
        ret
    }
    /// Constructs a matrix from a list of n^2 elements, in **column-major**
    /// order.
    ///
    /// ```
    /// # use ndpuzzle::math::{Matrix};
    /// # use ndpuzzle::row_matrix;
    /// assert_eq!(
    ///     Matrix::from_elems(vec![1, 2, 3, 4]),
    ///     row_matrix![
    ///         [1, 3],
    ///         [2, 4],
    ///     ],
    /// );
    /// ```
    pub fn from_elems(elems: Vec<N>) -> Self {
        let ndim = (elems.len() as f64).sqrt() as u8;
        assert_eq!(
            ndim as usize * ndim as usize,
            elems.len(),
            "matrix must have square number of elements; got {} elements",
            elems.len(),
        );
        Matrix { ndim, elems }
    }
    /// Constructs a matrix from a list of columns, where the number of columns
    /// determines the size of the matrix.
    pub fn from_cols<I>(cols: impl IntoIterator<IntoIter = I>) -> Self
    where
        I: ExactSizeIterator,
        I::Item: VectorRef<N>,
    {
        let cols = cols.into_iter();
        let ndim = cols.len() as u8;
        Self {
            ndim,
            elems: cols
                .flat_map(|col| (0..ndim).map(move |i| col.get(i)))
                .collect(),
        }
    }

    /// Constructs a matrix from the outer product of two vectors.
    pub fn from_outer_product(u: impl VectorRef<N>, v: impl VectorRef<N>) -> Self {
        let dim = std::cmp::max(u.ndim(), v.ndim());
        let u = &u;
        let v = &v;
        Self::from_elems(
            (0..dim)
                .flat_map(|i| (0..dim).map(move |j| u.get(i) * v.get(j)))
                .collect(),
        )
    }

    /// Contruct the matrix rotating in a plane from u to v.
    pub fn from_vec_to_vec(u: &impl VectorRef<N>, v: &impl VectorRef<N>) -> Self
    where
        N: Clone + Num + std::fmt::Debug,
    {
        let dim = std::cmp::max(u.ndim(), v.ndim());
        let tm = Matrix::from_outer_product(u, v);
        let tm = &tm - &tm.transpose();
        &(&Matrix::ident(dim) + &tm) + &((&tm * &tm).scale(N::one() / (N::one() + u.dot(v))))
    }

    /// Returns the number of dimensions (size) of the matrix.
    pub fn ndim(&self) -> u8 {
        self.ndim
    }

    /// Returns an element from the matrix. If either `col` or `row` is out of
    /// bounds, returns the corresponding element from the infinite identity
    /// matrix.
    pub fn get(&self, col: u8, row: u8) -> N {
        let ndim = self.ndim();
        if col < ndim && row < ndim {
            self.elems[col as usize * ndim as usize + row as usize].clone()
        } else if col == row {
            N::one()
        } else {
            N::zero()
        }
    }
    /// Returns a mutable reference to an element from the matrix.
    ///
    /// # Panics
    ///
    /// This method panics if `col >= self.ndim() || row >= self.ndim()`.
    pub fn get_mut(&mut self, col: u8, row: u8) -> &mut N {
        let ndim = self.ndim();
        assert!(col < ndim);
        assert!(row < ndim);
        &mut self.elems[col as usize * ndim as usize + row as usize]
    }
    /// Returns a row of the matrix. If out of bounds, returns the corresponding
    /// row of the infinite identity matrix.
    pub fn row(&self, row: u8) -> MatrixRow<'_, N> {
        MatrixRow { matrix: self, row }
    }
    /// Returns a column of the matrix. If out of bounds, returns the
    /// corresponding column of the infinite identity matrix.
    pub fn col(&self, col: u8) -> MatrixCol<'_, N> {
        MatrixCol { matrix: self, col }
    }

    /// Returns an iterator over the rows of the matrix.
    pub fn rows(&self) -> impl Iterator<Item = MatrixRow<'_, N>> {
        self.rows_ndim(self.ndim())
    }
    /// Returns an iterator over the columns of the matrix.
    pub fn cols(&self) -> impl Iterator<Item = MatrixCol<'_, N>> {
        self.cols_ndim(self.ndim())
    }
    /// Returns an iterator over the rows of the matrix, padded to `ndim`. Each
    /// individual row is not padded.
    pub fn rows_ndim(&self, ndim: u8) -> impl Iterator<Item = MatrixRow<'_, N>> {
        (0..ndim).map(|i| self.row(i))
    }
    /// Returns an iterator over the columns of the matrix, padded to `ndim`.
    /// Each individual column is not padded.
    pub fn cols_ndim(&self, ndim: u8) -> impl Iterator<Item = MatrixCol<'_, N>> {
        (0..ndim).map(|i| self.col(i))
    }

    /// Multiplies all elements of the matrix by a scalar.
    #[must_use]
    pub fn scale(mut self, scalar: N) -> Self {
        for elem in &mut self.elems {
            *elem = elem.clone() * scalar.clone();
        }
        self
    }

    /// Transforms a vector using the matrix.
    pub fn transform(&self, v: impl VectorRef<N>) -> Vector<N> {
        // TODO: remove this method; replace with `impl Mul`
        let ndim = std::cmp::max(self.ndim(), v.ndim());
        (0..ndim)
            .map(|i| {
                (0..ndim)
                    .map(|j| self.get(j, i) * v.get(j))
                    .fold(N::zero(), |a, b| a + b)
            })
            .collect()
    }

    /// Returns the determinant of the matrix.
    pub fn determinant(&self) -> N
    where
        N: Signed,
    {
        permutations::permutations_with_parity(0..self.ndim)
            .map(|(permutation, parity)| {
                let parity = match parity {
                    permutations::Parity::Even => N::one(),
                    permutations::Parity::Odd => -N::one(),
                };
                permutation
                    .into_iter()
                    .enumerate()
                    .map(|(j, k)| self.get(j as _, k))
                    .fold(N::one(), |x, y| x * y)
                    * parity
            })
            .fold(N::zero(), |x, y| x + y)
    }

    /// Returns the inverse of the matrix, or `None` if the determinant is zero.
    pub fn inverse(&self) -> Option<Matrix<N>>
    where
        N: Signed,
        N: Clone,
    {
        let determinant = self.determinant();
        (!determinant.is_zero()).then(|| {
            let det = &determinant;
            Matrix::from_elems(
                (0..self.ndim)
                    .flat_map(|j| {
                        (0..self.ndim).map(move |i| {
                            let mut a = self.clone();
                            for k in 0..self.ndim {
                                *a.get_mut(i, k) = N::zero();
                            }
                            *a.get_mut(i, j) = N::one();
                            a.determinant() / det.clone()
                        })
                    })
                    .collect(),
            )
        })
    }

    /// Returns the transpose of the matrix.
    pub fn transpose(&self) -> Matrix<N> {
        Matrix::from_cols(self.rows().collect::<Vec<_>>())
    }
}
impl<N: Clone + Num> FromIterator<N> for Matrix<N> {
    fn from_iter<T: IntoIterator<Item = N>>(iter: T) -> Self {
        Self::from_elems(iter.into_iter().collect())
    }
}

/// Constructs a matrix from columns.
#[macro_export]
macro_rules! col_matrix {
    ($([$($n:expr),* $(,)?]),* $(,)?) => {
        Matrix::from_elems(vec![$($($n),*),*])
    };
}
/// Constructs a matrix from rows.
#[macro_export]
macro_rules! row_matrix {
    ($([$($n:expr),* $(,)?]),* $(,)?) => {
        Matrix::from_elems(vec![$($($n),*),*]).transpose()
    };
}

/// Reference to a column of a matrix, usable as a vector.
#[derive(Debug, Copy, Clone)]
pub struct MatrixCol<'a, N: Clone + Num> {
    matrix: &'a Matrix<N>,
    col: u8,
}
impl<N: Clone + Num> VectorRef<N> for MatrixCol<'_, N> {
    fn ndim(&self) -> u8 {
        std::cmp::max(self.matrix.ndim(), self.col + 1)
    }

    fn get(&self, row: u8) -> N {
        self.matrix.get(self.col, row)
    }
}

/// Reference to a row of a matrix, usable as a vector.
#[derive(Debug, Copy, Clone)]
pub struct MatrixRow<'a, N: Clone + Num> {
    matrix: &'a Matrix<N>,
    row: u8,
}
impl<N: Clone + Num> VectorRef<N> for MatrixRow<'_, N> {
    fn ndim(&self) -> u8 {
        std::cmp::max(self.matrix.ndim(), self.row + 1)
    }

    fn get(&self, col: u8) -> N {
        self.matrix.get(col, self.row)
    }
}

impl_vector_ops!(impl<N> for MatrixCol<'_, N>);
impl_vector_ops!(impl<N> for MatrixRow<'_, N>);

impl<'a, N: Clone + Num + std::fmt::Debug> Mul for &'a Matrix<N> {
    type Output = Matrix<N>;

    fn mul(self, rhs: Self) -> Self::Output {
        let new_ndim = std::cmp::max(self.ndim(), rhs.ndim());
        let mut new_matrix = Matrix::zero(new_ndim);

        for i in 0..new_ndim {
            let self_col = self.col(i);
            for x in 0..new_ndim {
                let rhs_elem = rhs.get(x, i as _);
                for y in 0..new_ndim {
                    let self_elem = self_col.get(y);
                    *new_matrix.get_mut(x, y) =
                        new_matrix.get(x, y) + self_elem.clone() * rhs_elem.clone();
                }
            }
        }

        new_matrix
    }
}
impl<'a, N: Clone + Num + std::fmt::Debug> Add for &'a Matrix<N> {
    type Output = Matrix<N>;

    fn add(self, rhs: Self) -> Self::Output {
        let new_ndim = std::cmp::max(self.ndim(), rhs.ndim());
        Matrix::from_elems(
            (0..new_ndim)
                .flat_map(|i| (0..new_ndim).map(move |j| self.get(i, j) + rhs.get(i, j)))
                .collect(),
        )
    }
}
impl<'a, N: Clone + Num + std::fmt::Debug> Sub for &'a Matrix<N> {
    type Output = Matrix<N>;

    fn sub(self, rhs: Self) -> Self::Output {
        let new_ndim = std::cmp::max(self.ndim(), rhs.ndim());
        Matrix::from_elems(
            (0..new_ndim)
                .flat_map(|i| (0..new_ndim).map(move |j| self.get(i, j) - rhs.get(i, j)))
                .collect(),
        )
    }
}
impl Matrix<f32> {
    /// Returns whether two matrices are equal within `epsilon` on each element.
    pub fn approx_eq(&self, other: &Self, epsilon: f32) -> bool {
        let ndim = std::cmp::max(self.ndim(), other.ndim());
        self.cols_ndim(ndim)
            .zip(other.cols_ndim(ndim))
            .all(|(a, b)| a.approx_eq(b, epsilon))
    }
}

impl_forward_bin_ops_to_ref!(impl Mul for Matrix<f32> { fn mul() });
impl_forward_bin_ops_to_ref!(impl Add for Matrix<f32> { fn add() });
impl_forward_bin_ops_to_ref!(impl Sub for Matrix<f32> { fn sub() });

impl Mul<f32> for Matrix<f32> {
    type Output = Matrix<f32>;

    fn mul(mut self, rhs: f32) -> Self::Output {
        for x in &mut self.elems {
            *x *= rhs;
        }
        self
    }
}
impl Div<f32> for Matrix<f32> {
    type Output = Matrix<f32>;

    fn div(self, rhs: f32) -> Self::Output {
        self * (1.0 / rhs)
    }
}

impl<V: VectorRef<f32>> Mul<V> for Matrix<f32> {
    type Output = Vector<f32>;

    fn mul(self, rhs: V) -> Self::Output {
        self.transform(rhs)
    }
}
impl<'a, V: VectorRef<f32>> Mul<V> for &'a Matrix<f32> {
    type Output = Vector<f32>;

    fn mul(self, rhs: V) -> Self::Output {
        self.transform(rhs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_matrix_empty_ident() {
        assert_eq!(
            Matrix::EMPTY_IDENT * vector![1.0, 2.0, 3.0, 4.0],
            vector![1.0, 2.0, 3.0, 4.0],
        );
        let m = col_matrix![[1.0, 2.0, 3.0], [4.0, 5.0, 6.0], [7.0, 8.0, 9.0]];
        assert_eq!(m, Matrix::EMPTY_IDENT * &m);
        assert_eq!(m, &m * Matrix::EMPTY_IDENT);
    }

    #[test]
    fn test_matrix_multiply() {
        let m1 = col_matrix![[1, 2, 0, 0], [0, 1, 1, 0], [1, 1, 1, 0], [0, 0, 0, -3]];
        let m2 = col_matrix![[1, 2, 4], [2, 3, 2], [1, 1, 2]];
        assert_eq!(
            &m1 * &m2,
            col_matrix![[5, 8, 6, 0], [4, 9, 5, 0], [3, 5, 3, 0], [0, 0, 0, -3]],
        );
    }

    #[test]
    fn test_matrix_determinant() {
        let m = col_matrix![[3, 7], [1, -4]];
        assert_eq!(m.determinant(), -19);

        let m = col_matrix![[-2, -1, 2], [2, 1, 4], [-3, 3, -1]];
        assert_eq!(m.determinant(), 54);

        let m = col_matrix![[1, 2, 3, 4], [5, 6, 8, 7], [-10, 3, 6, 2], [3, 1, 4, 1]];
        assert_eq!(m.determinant(), -402);
    }

    #[test]
    fn test_matrix_inverse() {
        let m = col_matrix![[1., 0., 4.], [1., 1., 6.], [-3., 0., -10.]];
        assert_eq!(&m * &m.inverse().unwrap(), Matrix::ident(3));
    }

    #[test]
    fn test_matrix_transpose() {
        assert_eq!(
            row_matrix![[1, 2, 3], [4, 5, 6], [7, 8, 9]],
            col_matrix![[1, 4, 7], [2, 5, 8], [3, 6, 9]],
        )
    }
}
