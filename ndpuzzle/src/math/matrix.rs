//! N-dimensional matrix math.

use std::ops::*;

use super::permutations;
use super::vector::{Vector, VectorRef};

/// N-by-N square matrix. Indexing out of bounds returns the corresponding
/// element from the infinite identity matrix.
#[derive(Debug, Clone, PartialEq)]
pub struct Matrix {
    /// Number of dimensions of the matrix.
    ndim: u8,
    /// Elements stored in **column-major** order.
    elems: Vec<f32>,
}
impl Matrix {
    /// 0-by-0 matrix that functions as the identity matrix.
    pub const EMPTY_IDENT: Self = Matrix {
        ndim: 0,
        elems: vec![],
    };

    /// Constructs a matrix with all zeros.
    pub fn zero(ndim: u8) -> Self {
        Self {
            ndim,
            elems: vec![0.0; ndim as usize * ndim as usize],
        }
    }
    /// Constructs an identity matrix.
    pub fn ident(ndim: u8) -> Self {
        let mut ret = Self::zero(ndim);
        for i in 0..ndim {
            *ret.get_mut(i, i) = 1.0;
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
    pub fn from_elems(elems: Vec<f32>) -> Self {
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
        I::Item: VectorRef,
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
    /// Constructs a matrix from a function for each element.
    pub fn from_fn(ndim: u8, mut f: impl FnMut(u8, u8) -> f32) -> Self {
        (0..ndim).flat_map(|i| (0..ndim).map(|j| f(i, j))).collect()
    }

    /// Constructs a matrix from the outer product of two vectors.
    pub fn from_outer_product(u: impl VectorRef, v: impl VectorRef) -> Self {
        let dim = std::cmp::max(u.ndim(), v.ndim());
        let u = &u;
        let v = &v;
        Self::from_elems(
            (0..dim)
                .flat_map(|i| (0..dim).map(move |j| u.get(i) * v.get(j)))
                .collect(),
        )
    }

    /// Contructs the matrix rotating in a plane from `u` to `v`. Both vectors
    /// are assumed to be normalized.
    pub fn from_vec_to_vec(u: impl VectorRef, v: impl VectorRef) -> Self {
        let dim = std::cmp::max(u.ndim(), v.ndim());
        let tm = Matrix::from_outer_product(&u, &v);
        let tm = &tm - tm.transpose();
        (Matrix::ident(dim) + &tm) + (&tm * &tm) / (1.0 + u.dot(v))
    }
    /// Constructs the matrix reflecting through `v`, which is assumed to be
    /// normalized.
    pub fn from_reflection(v: impl VectorRef) -> Self {
        // source: Wikipedia (https://w.wiki/5mmn)
        Self::from_fn(v.ndim(), |i, j| {
            (i == j) as u8 as f32 - 2.0 * v.get(i) * v.get(j)
        })
    }

    /// Returns the number of dimensions (size) of the matrix.
    pub fn ndim(&self) -> u8 {
        self.ndim
    }

    /// Pads the matrix with identity up to `ndim`.
    #[must_use]
    pub fn pad(&self, ndim: u8) -> Matrix {
        if ndim <= self.ndim() {
            self.clone()
        } else {
            let mut ret = Matrix::ident(ndim);
            for i in 0..self.ndim() {
                for j in 0..self.ndim() {
                    *ret.get_mut(i, j) = self.get(i, j);
                }
            }
            ret
        }
    }

    /// Returns an element from the matrix. If either `col` or `row` is out of
    /// bounds, returns the corresponding element from the infinite identity
    /// matrix.
    pub fn get(&self, col: u8, row: u8) -> f32 {
        let ndim = self.ndim();
        if col < ndim && row < ndim {
            self.elems[col as usize * ndim as usize + row as usize]
        } else if col == row {
            1.0
        } else {
            0.0
        }
    }
    /// Returns a mutable reference to an element from the matrix.
    ///
    /// # Panics
    ///
    /// This method panics if `col >= self.ndim() || row >= self.ndim()`.
    pub fn get_mut(&mut self, col: u8, row: u8) -> &mut f32 {
        let ndim = self.ndim();
        assert!(col < ndim);
        assert!(row < ndim);
        &mut self.elems[col as usize * ndim as usize + row as usize]
    }
    /// Returns a row of the matrix. If out of bounds, returns the corresponding
    /// row of the infinite identity matrix.
    pub fn row(&self, row: u8) -> MatrixRow<'_> {
        MatrixRow { matrix: self, row }
    }
    /// Returns a column of the matrix. If out of bounds, returns the
    /// corresponding column of the infinite identity matrix.
    pub fn col(&self, col: u8) -> MatrixCol<'_> {
        MatrixCol { matrix: self, col }
    }

    /// Returns an iterator over the rows of the matrix.
    pub fn rows(&self) -> impl Iterator<Item = MatrixRow<'_>> {
        self.rows_ndim(self.ndim())
    }
    /// Returns an iterator over the columns of the matrix.
    pub fn cols(&self) -> impl Iterator<Item = MatrixCol<'_>> {
        self.cols_ndim(self.ndim())
    }
    /// Returns an iterator over the rows of the matrix, padded to `ndim`. Each
    /// individual row is not padded.
    pub fn rows_ndim(&self, ndim: u8) -> impl Iterator<Item = MatrixRow<'_>> {
        (0..ndim).map(|i| self.row(i))
    }
    /// Returns an iterator over the columns of the matrix, padded to `ndim`.
    /// Each individual column is not padded.
    pub fn cols_ndim(&self, ndim: u8) -> impl Iterator<Item = MatrixCol<'_>> {
        (0..ndim).map(|i| self.col(i))
    }

    /// Returns the determinant of the matrix.
    pub fn determinant(&self) -> f32 {
        permutations::permutations_with_parity(0..self.ndim)
            .map(|(permutation, parity)| {
                let parity = match parity {
                    permutations::Parity::Even => 1.0,
                    permutations::Parity::Odd => -1.0,
                };
                permutation
                    .into_iter()
                    .enumerate()
                    .map(|(j, k)| self.get(j as _, k))
                    .product::<f32>()
                    * parity
            })
            .sum()
    }

    /// Returns the inverse of the matrix, or `None` if the determinant is zero.
    pub fn inverse(&self) -> Option<Matrix> {
        let determinant = self.determinant();
        let recip_determinant = 1.0 / determinant;
        recip_determinant.is_finite().then(|| {
            Matrix::from_elems(
                (0..self.ndim)
                    .flat_map(|j| {
                        (0..self.ndim).map(move |i| {
                            let mut a = self.clone();
                            for k in 0..self.ndim {
                                *a.get_mut(i, k) = 0.0;
                            }
                            *a.get_mut(i, j) = 1.0;
                            a.determinant() * recip_determinant
                        })
                    })
                    .collect(),
            )
        })
    }

    /// Returns the transpose of the matrix.
    pub fn transpose(&self) -> Matrix {
        Matrix::from_cols((0..self.ndim()).map(|i| self.row(i)))
    }
}
impl FromIterator<f32> for Matrix {
    fn from_iter<T: IntoIterator<Item = f32>>(iter: T) -> Self {
        Self::from_elems(iter.into_iter().collect())
    }
}

/// Constructs a matrix from columns.
#[macro_export]
macro_rules! col_matrix {
    ($([$($n:expr),* $(,)?]),* $(,)?) => {
        Matrix::from_elems(vec![$($($n as f32),*),*])
    };
}
/// Constructs a matrix from rows.
#[macro_export]
macro_rules! row_matrix {
    ($([$($n:expr),* $(,)?]),* $(,)?) => {
        Matrix::from_elems(vec![$($($n as f32),*),*]).transpose()
    };
}

/// Reference to a column of a matrix, usable as a vector.
#[derive(Debug, Copy, Clone)]
pub struct MatrixCol<'a> {
    matrix: &'a Matrix,
    col: u8,
}
impl VectorRef for MatrixCol<'_> {
    fn ndim(&self) -> u8 {
        std::cmp::max(self.matrix.ndim(), self.col + 1)
    }

    fn get(&self, row: u8) -> f32 {
        self.matrix.get(self.col, row)
    }
}
impl PartialEq for MatrixCol<'_> {
    fn eq(&self, other: &Self) -> bool {
        let ndim = std::cmp::max(self.ndim(), other.ndim());
        self.iter_ndim(ndim).eq(other.iter_ndim(ndim))
    }
}
impl approx::AbsDiffEq for MatrixCol<'_> {
    type Epsilon = f32;

    fn default_epsilon() -> Self::Epsilon {
        super::EPSILON
    }

    fn abs_diff_eq(&self, other: &Self, epsilon: Self::Epsilon) -> bool {
        let ndim = std::cmp::max(self.ndim(), other.ndim());
        self.iter_ndim(ndim)
            .zip(other.iter_ndim(ndim))
            .all(|(a, b)| a.abs_diff_eq(&b, epsilon))
    }
}

/// Reference to a row of a matrix, usable as a vector.
#[derive(Debug, Copy, Clone)]
pub struct MatrixRow<'a> {
    matrix: &'a Matrix,
    row: u8,
}
impl VectorRef for MatrixRow<'_> {
    fn ndim(&self) -> u8 {
        std::cmp::max(self.matrix.ndim(), self.row + 1)
    }

    fn get(&self, col: u8) -> f32 {
        self.matrix.get(col, self.row)
    }
}
impl PartialEq for MatrixRow<'_> {
    fn eq(&self, other: &Self) -> bool {
        let ndim = std::cmp::max(self.ndim(), other.ndim());
        self.iter_ndim(ndim).eq(other.iter_ndim(ndim))
    }
}
impl approx::AbsDiffEq for MatrixRow<'_> {
    type Epsilon = f32;

    fn default_epsilon() -> Self::Epsilon {
        super::EPSILON
    }

    fn abs_diff_eq(&self, other: &Self, epsilon: Self::Epsilon) -> bool {
        self.approx_eq(other, epsilon)
    }
}

impl_vector_ops!(impl for MatrixCol<'_, >);
impl_vector_ops!(impl for MatrixRow<'_, >);

impl<'a> Mul for &'a Matrix {
    type Output = Matrix;

    fn mul(self, rhs: Self) -> Self::Output {
        let new_ndim = std::cmp::max(self.ndim(), rhs.ndim());
        let mut new_matrix = Matrix::zero(new_ndim);

        for i in 0..new_ndim {
            let self_col = self.col(i);
            for x in 0..new_ndim {
                let rhs_elem = rhs.get(x, i as _);
                for y in 0..new_ndim {
                    let self_elem = self_col.get(y);
                    *new_matrix.get_mut(x, y) = new_matrix.get(x, y) + self_elem * rhs_elem;
                }
            }
        }

        new_matrix
    }
}
impl<'a> Add for &'a Matrix {
    type Output = Matrix;

    fn add(self, rhs: Self) -> Self::Output {
        let new_ndim = std::cmp::max(self.ndim(), rhs.ndim());
        Matrix::from_fn(new_ndim, |i, j| self.get(i, j) + rhs.get(i, j))
    }
}
impl<'a> Sub for &'a Matrix {
    type Output = Matrix;

    fn sub(self, rhs: Self) -> Self::Output {
        let new_ndim = std::cmp::max(self.ndim(), rhs.ndim());
        Matrix::from_fn(new_ndim, |i, j| self.get(i, j) - rhs.get(i, j))
    }
}

impl approx::AbsDiffEq for Matrix {
    type Epsilon = f32;

    fn default_epsilon() -> Self::Epsilon {
        super::EPSILON
    }

    fn abs_diff_eq(&self, other: &Self, epsilon: Self::Epsilon) -> bool {
        let ndim = std::cmp::max(self.ndim(), other.ndim());
        let self_cols = self.cols_ndim(ndim);
        let other_cols = other.cols_ndim(ndim);
        self_cols
            .zip(other_cols)
            .all(|(a, b)| a.abs_diff_eq(&b, epsilon))
    }
}

impl_forward_bin_ops_to_ref! {
    impl Mul for Matrix { fn mul() }
    impl Add for Matrix { fn add() }
    impl Sub for Matrix { fn sub() }
}

impl Mul<f32> for Matrix {
    type Output = Matrix;

    fn mul(mut self, rhs: f32) -> Self::Output {
        for x in &mut self.elems {
            *x *= rhs;
        }
        self
    }
}
impl<'a> Mul<f32> for &'a Matrix {
    type Output = Matrix;

    fn mul(self, rhs: f32) -> Self::Output {
        Matrix::from_elems(self.elems.iter().map(|&x| x * rhs).collect())
    }
}

impl Div<f32> for Matrix {
    type Output = Matrix;

    fn div(self, rhs: f32) -> Self::Output {
        self * (1.0 / rhs)
    }
}
impl<'a> Div<f32> for &'a Matrix {
    type Output = Matrix;

    fn div(self, rhs: f32) -> Self::Output {
        self * (1.0 / rhs)
    }
}

impl<V: VectorRef> Mul<V> for Matrix {
    type Output = Vector;

    fn mul(self, rhs: V) -> Self::Output {
        &self * rhs
    }
}
impl<'a, V: VectorRef> Mul<V> for &'a Matrix {
    type Output = Vector;

    fn mul(self, rhs: V) -> Self::Output {
        let ndim = std::cmp::max(self.ndim(), rhs.ndim());
        (0..ndim)
            .map(|i| (0..ndim).map(|j| self.get(j, i) * rhs.get(j)).sum())
            .collect()
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
        let m = col_matrix![[1, 2, 3], [4, 5, 6], [7, 8, 9]];
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
        assert_eq!(m.determinant(), -19.0);

        let m = col_matrix![[-2, -1, 2], [2, 1, 4], [-3, 3, -1]];
        assert_eq!(m.determinant(), 54.0);

        let m = col_matrix![[1, 2, 3, 4], [5, 6, 8, 7], [-10, 3, 6, 2], [3, 1, 4, 1]];
        assert_eq!(m.determinant(), -402.0);
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
