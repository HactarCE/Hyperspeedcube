use itertools::Itertools;
use num_traits::{Float, Num};
use std::fmt;
use std::iter::Cloned;
use std::marker::PhantomData;
use std::ops::*;

/// N-dimensional vector. Indexing out of bounds returns zero.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Vector<N: Clone + Num>(pub Vec<N>);

/// Reference to an N-dimensional vector. Indexing out of bounds returns zero.
pub trait VectorRef<N: Clone + Num>: Sized {
    /// Returns the number of components in the vector.
    fn ndim(&self) -> u8;

    /// Returns a component of the vector. If the index is out of bounds,
    /// returns zero.
    fn get(&self, idx: u8) -> N;

    /// Returns an iterator over the components of the vector.
    fn iter(&self) -> VectorIter<'_, N, Self> {
        VectorIter {
            range: 0..self.ndim(),
            vector: self,
            _phantom: PhantomData,
        }
    }

    /// Returns the dot product of this vector with another.
    fn dot(&self, rhs: impl VectorRef<N>) -> N {
        self.iter()
            .zip(rhs.iter())
            .map(|(l, r)| l * r)
            .fold(N::zero(), |l, r| l + r)
    }

    /// Pads the vector with zeros up to `ndim`.
    #[must_use]
    fn pad(&self, ndim: u8) -> Vector<N> {
        self.iter()
            .pad_using(ndim as usize, |_| N::zero())
            .collect()
    }

    /// Returns the magnitude of the vector.
    fn mag(&self) -> N
    where
        N: Float,
    {
        self.mag2().sqrt()
    }
    /// Returns the squared magnitude of the vector.
    fn mag2(&self) -> N {
        self.dot(self)
    }

    /// Returns whether two vectors are equal within `epsilon` on each
    /// component.
    fn approx_eq(&self, other: impl VectorRef<N>, epsilon: f32) -> bool
    where
        N: Into<f32>,
    {
        let ndim = std::cmp::max(self.ndim(), other.ndim()) as usize;
        let self_xs = self.iter().map(|x| x.into()).pad_using(ndim, |_| 0.0);
        let other_xs = other.iter().map(|x| x.into()).pad_using(ndim, |_| 0.0);
        self_xs.zip(other_xs).all(|(l, r)| (l - r).abs() <= epsilon)
    }
}

/// Iterator over the components of a vector.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct VectorIter<'a, N: Clone + Num, V: VectorRef<N>> {
    range: Range<u8>,
    vector: &'a V,
    _phantom: PhantomData<N>,
}
impl<N: Clone + Num, V: VectorRef<N>> Iterator for VectorIter<'_, N, V> {
    type Item = N;

    fn next(&mut self) -> Option<Self::Item> {
        self.range.next().map(|i| self.vector.get(i))
    }
}
impl<N: Clone + Num> VectorRef<N> for Vector<N> {
    fn ndim(&self) -> u8 {
        self.0.len() as _
    }

    fn get(&self, idx: u8) -> N {
        self.0.get(idx as usize).cloned().unwrap_or(N::zero())
    }
}

impl<N: Clone + Num, V: VectorRef<N>> VectorRef<N> for &'_ V {
    fn ndim(&self) -> u8 {
        (*self).ndim()
    }

    fn get(&self, idx: u8) -> N {
        (*self).get(idx)
    }
}

impl<N: Clone + Num + fmt::Display> fmt::Display for Vector<N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "(")?;
        let mut iter = self.0.iter();
        if let Some(first) = iter.next() {
            write!(f, "{first}")?;
            for elem in iter {
                write!(f, ", {elem}")?;
            }
        }
        write!(f, ")")?;
        Ok(())
    }
}

/// Constructs an N-dimensional vector, using the same syntax as `vec![]`.
#[macro_export]
macro_rules! vector {
    [$($tok:tt)*] => {
        Vector(vec![$($tok)*])
    };
}

macro_rules! impl_zero_padded_op {
    (impl<$num:ident> $trait_name:ident for $type_name:ty { fn $fn_name:ident() }) => {
        impl<$num: Clone + Num, T: VectorRef<$num>> $trait_name<T> for $type_name {
            type Output = Vector<$num>;

            fn $fn_name(self, rhs: T) -> Self::Output {
                use itertools::Itertools;

                let result_ndim = std::cmp::max(self.ndim(), rhs.ndim());
                let lhs = self.iter().pad_using(result_ndim as _, |_| N::zero());
                let rhs = rhs.iter().pad_using(result_ndim as _, |_| N::zero());
                lhs.zip(rhs).map(|(l, r)| l.$fn_name(r)).collect()
            }
        }
    };
}
macro_rules! impl_vector_ops {
    (impl<$num:ident> for $type_name:ty) => {
        impl_zero_padded_op!(impl<$num> Add for $type_name { fn add() });
        impl_zero_padded_op!(impl<$num> Sub for $type_name { fn sub() });

        impl<$num: Clone + num_traits::Signed> Neg for $type_name {
            type Output = Vector<N>;

            fn neg(self) -> Self::Output {
                self.iter().map(|n| -n).collect()
            }
        }

        impl<$num: Clone + Num> Mul<$num> for $type_name {
            type Output = Vector<$num>;

            fn mul(self, rhs: $num) -> Self::Output {
                self.iter().map(|x| x * rhs.clone()).collect()
            }
        }
        impl<$num: Clone + Num> Div<$num> for $type_name {
            type Output = Vector<$num>;

            fn div(self, rhs: $num) -> Self::Output {
                self.iter().map(|x| x / rhs.clone()).collect()
            }
        }
    };
}
impl_vector_ops!(impl<N> for Vector<N>);
impl_vector_ops!(impl<N> for &'_ Vector<N>);

impl<N: Clone + Num> Index<u8> for Vector<N> {
    type Output = N;

    fn index(&self, index: u8) -> &Self::Output {
        &self.0[index as usize]
    }
}
impl<N: Clone + Num> Index<u8> for &'_ Vector<N> {
    type Output = N;

    fn index(&self, index: u8) -> &Self::Output {
        &self.0[index as usize]
    }
}
impl<N: Clone + Num> IndexMut<u8> for Vector<N> {
    fn index_mut(&mut self, index: u8) -> &mut Self::Output {
        let ndim = self.ndim();
        self.0.get_mut(index as usize).unwrap_or_else(|| {
            panic!(
                "vector index out of bounds: the dimensionality is {} but the index is {}",
                ndim, index,
            )
        })
    }
}

impl<N: Clone + Num> Vector<N> {
    /// Zero-dimensional empty vector.
    pub const EMPTY: Self = Self(vec![]);

    /// Returns a unit vector along an axis.
    pub fn unit(axis: u8) -> Self {
        let mut ret = vector![N::zero(); axis as usize+1];
        ret[axis] = N::one();
        ret
    }

    /// Returns an iterator over the components of the vector.
    pub fn iter(&self) -> impl '_ + Iterator<Item = N> {
        self.0.iter().cloned()
    }
    /// Returns an iterator over the components of the vector, padded to `ndim`.
    pub fn iter_ndim(&self, ndim: u8) -> impl '_ + Iterator<Item = N> {
        self.iter()
            .pad_using(ndim as _, |_| N::zero())
            .take(ndim as _)
    }
}

impl<N: Clone + Num> IntoIterator for Vector<N> {
    type Item = N;

    type IntoIter = std::vec::IntoIter<N>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}
impl<'a, N: Clone + Num> IntoIterator for &'a Vector<N> {
    type Item = N;

    type IntoIter = Cloned<std::slice::Iter<'a, N>>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter().cloned()
    }
}

impl<N: Clone + Num> FromIterator<N> for Vector<N> {
    fn from_iter<T: IntoIterator<Item = N>>(iter: T) -> Self {
        Self(iter.into_iter().collect())
    }
}

impl Vector<f32> {
    /// Resizes the vector in-place, padding with zeros.
    #[must_use]
    pub fn resize(mut self, ndim: u8) -> Self {
        self.0.resize(ndim as _, 0.0);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    pub fn test_vector_add() {
        let v1 = vector![1, 2, -10];
        let v2 = vector![-5];
        assert_eq!(&v1 + &v2, vector![-4, 2, -10]);
        assert_eq!(v2 + v1, vector![-4, 2, -10]);
    }

    #[test]
    pub fn test_vector_sub() {
        let v1 = vector![1, 2, -10];
        let v2 = vector![-5];
        assert_eq!(&v1 - &v2, vector![6, 2, -10]);
        assert_eq!(v2 - &v1, vector![-6, -2, 10]);
    }

    #[test]
    pub fn test_vector_neg() {
        let v1 = vector![1, 2, -10];
        assert_eq!(-&v1, vector![-1, -2, 10]);
        assert_eq!(-v1, vector![-1, -2, 10]);
    }

    #[test]
    pub fn test_dot_product() {
        let v1 = vector![1, 2, -10];
        let v2 = vector![-5, 16];
        assert_eq!(v1.dot(v2), 27);
    }
}
