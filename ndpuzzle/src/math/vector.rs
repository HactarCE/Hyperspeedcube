//! N-dimensional vector math.

use itertools::Itertools;
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;
use std::fmt;
use std::iter::Sum;
use std::ops::*;

/// N-dimensional vector. Indexing out of bounds returns zero.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(transparent)]
pub struct Vector(pub SmallVec<[f32; 4]>);

/// Reference to an N-dimensional vector. Indexing out of bounds returns zero.
pub trait VectorRef: Sized + fmt::Debug {
    /// Converts the vector to a `Vector`.
    fn to_vector(&self) -> Vector {
        self.iter().collect()
    }

    /// Returns the number of components in the vector.
    fn ndim(&self) -> u8;

    /// Returns a component of the vector. If the index is out of bounds,
    /// returns zero.
    fn get(&self, idx: u8) -> f32;

    /// Returns an iterator over the components of the vector.
    fn iter(&self) -> VectorIter<&Self> {
        self.iter_ndim(self.ndim())
    }
    /// Returns an iterator over the components of the vector, padded to `ndim`.
    fn iter_ndim(&self, ndim: u8) -> VectorIter<&Self> {
        VectorIter {
            range: 0..ndim,
            vector: self,
        }
    }

    /// Returns the dot product of this vector with another.
    fn dot(&self, rhs: impl VectorRef) -> f32 {
        self.iter().zip(rhs.iter()).map(|(l, r)| l * r).sum()
    }

    /// Pads the vector with zeros up to `ndim`.
    #[must_use]
    fn pad(&self, ndim: u8) -> Vector {
        self.iter().pad_using(ndim as usize, |_| 0.0).collect()
    }

    /// Returns the magnitude of the vector.
    fn mag(&self) -> f32 {
        self.mag2().sqrt()
    }
    /// Returns the squared magnitude of the vector.
    fn mag2(&self) -> f32 {
        self.dot(self)
    }

    /// Returns a normalized copy of the vector.
    #[must_use]
    fn normalize(&self) -> Option<Vector> {
        let mult = 1.0 / self.mag();
        mult.is_finite().then(|| self.scale(mult))
    }
    /// Returns a scaled copy of the vector.
    #[must_use]
    fn scale(&self, scalar: f32) -> Vector {
        self.iter().map(|x| x * scalar).collect()
    }

    /// Returns whether two vectors are equal within `epsilon` on each
    /// component.
    fn approx_eq(&self, other: impl VectorRef, epsilon: f32) -> bool {
        let ndim = std::cmp::max(self.ndim(), other.ndim());
        let self_xs = self.iter_ndim(ndim);
        let other_xs = other.iter_ndim(ndim);
        self_xs.zip(other_xs).all(|(l, r)| (l - r).abs() <= epsilon)
    }
}

/// Iterator over the components of a vector.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct VectorIter<V: VectorRef> {
    range: Range<u8>,
    vector: V,
}
impl<V: VectorRef> Iterator for VectorIter<V> {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        self.range.next().map(|i| self.vector.get(i))
    }
}
impl VectorRef for Vector {
    fn ndim(&self) -> u8 {
        self.0.len() as _
    }

    fn get(&self, idx: u8) -> f32 {
        self.0.get(idx as usize).cloned().unwrap_or(0.0)
    }
}

impl<V: VectorRef> VectorRef for &'_ V {
    fn ndim(&self) -> u8 {
        (*self).ndim()
    }

    fn get(&self, idx: u8) -> f32 {
        (*self).get(idx)
    }
}

impl fmt::Display for Vector {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "(")?;
        let mut iter = self.0.iter();
        if let Some(first) = iter.next() {
            first.fmt(f)?;
            for elem in iter {
                write!(f, ", ")?;
                elem.fmt(f)?;
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
        $crate::math::Vector(smallvec::smallvec![$($tok)*])
    };
}

macro_rules! impl_zero_padded_op {
    (impl $trait_name:ident for $type_name:ty { fn $fn_name:ident() }) => {
        impl<V: VectorRef> $trait_name<V> for $type_name {
            type Output = Vector;

            fn $fn_name(self, rhs: V) -> Self::Output {
                let ndim = std::cmp::max(self.ndim(), rhs.ndim());
                let lhs = self.iter_ndim(ndim);
                let rhs = rhs.iter_ndim(ndim);
                lhs.zip(rhs).map(|(l, r)| l.$fn_name(r)).collect()
            }
        }
    };
}
macro_rules! impl_vector_ops {
    (impl for $type_name:ty) => {
        impl_zero_padded_op!(impl Add for $type_name { fn add() });
        impl_zero_padded_op!(impl Sub for $type_name { fn sub() });

        impl Neg for $type_name {
            type Output = Vector;

            fn neg(self) -> Self::Output {
                self.iter().map(|n| -n).collect()
            }
        }

        impl Mul<f32> for $type_name {
            type Output = Vector;

            fn mul(self, rhs: f32) -> Self::Output {
                self.iter().map(|x| x * rhs).collect()
            }
        }
        impl Div<f32> for $type_name {
            type Output = Vector;

            #[allow(clippy::suspicious_arithmetic_impl)]
            fn div(self, rhs: f32) -> Self::Output {
                let mult = 1.0 / rhs;
                self.iter().map(|x| x * mult).collect()
            }
        }
    };
}
impl_vector_ops!(impl for Vector);
impl_vector_ops!(impl for &'_ Vector);

impl<V: VectorRef> AddAssign<V> for Vector {
    fn add_assign(&mut self, rhs: V) {
        let ndim = std::cmp::max(self.ndim(), rhs.ndim());
        self.0.resize(ndim as _, 0.0);
        for i in 0..rhs.ndim() {
            self[i] += rhs.get(i);
        }
    }
}
impl<V: VectorRef> MulAssign<V> for Vector {
    fn mul_assign(&mut self, rhs: V) {
        self.0.truncate(rhs.ndim() as _);
        for i in 0..rhs.ndim() {
            self[i] *= rhs.get(i);
        }
    }
}

impl Index<u8> for Vector {
    type Output = f32;

    fn index(&self, index: u8) -> &Self::Output {
        &self.0[index as usize]
    }
}
impl Index<u8> for &'_ Vector {
    type Output = f32;

    fn index(&self, index: u8) -> &Self::Output {
        &self.0[index as usize]
    }
}
impl IndexMut<u8> for Vector {
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

impl Vector {
    /// Zero-dimensional empty vector.
    pub const EMPTY: Self = Self(SmallVec::new_const());

    /// Returns a zero vector.
    pub fn zero(ndim: u8) -> Self {
        Self::EMPTY.resize(ndim)
    }
    /// Returns a unit vector along an axis.
    pub fn unit(axis: u8) -> Self {
        let mut ret = vector![0.0; axis as usize+1];
        ret[axis] = 1.0;
        ret
    }

    /// Resizes the vector in-place, padding with zeros.
    #[must_use]
    pub fn resize(mut self, ndim: u8) -> Self {
        self.0.resize(ndim as _, 0.0);
        self
    }
}
impl approx::AbsDiffEq for Vector {
    type Epsilon = f32;

    fn default_epsilon() -> Self::Epsilon {
        super::EPSILON
    }

    fn abs_diff_eq(&self, other: &Self, epsilon: Self::Epsilon) -> bool {
        self.approx_eq(other, epsilon)
    }
}

impl FromIterator<f32> for Vector {
    fn from_iter<T: IntoIterator<Item = f32>>(iter: T) -> Self {
        Self(iter.into_iter().collect())
    }
}

impl<V: VectorRef> Sum<V> for Vector {
    fn sum<I: Iterator<Item = V>>(iter: I) -> Self {
        let mut ret = Self::EMPTY;
        for v in iter {
            ret = ret.pad(v.ndim());
            ret += v;
        }
        ret
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    pub fn test_vector_add() {
        let v1 = vector![1.0, 2.0, -10.0];
        let v2 = vector![-5.0];
        assert_eq!(&v1 + &v2, vector![-4.0, 2.0, -10.0]);
        assert_eq!(v2 + v1, vector![-4.0, 2.0, -10.0]);
    }

    #[test]
    pub fn test_vector_sub() {
        let v1 = vector![1.0, 2.0, -10.0];
        let v2 = vector![-5.0];
        assert_eq!(&v1 - &v2, vector![6.0, 2.0, -10.0]);
        assert_eq!(v2 - &v1, vector![-6.0, -2.0, 10.0]);
    }

    #[test]
    pub fn test_vector_neg() {
        let v1 = vector![1.0, 2.0, -10.0];
        assert_eq!(-&v1, vector![-1.0, -2.0, 10.0]);
        assert_eq!(-v1, vector![-1.0, -2.0, 10.0]);
    }

    #[test]
    pub fn test_dot_product() {
        let v1 = vector![1.0, 2.0, -10.0];
        let v2 = vector![-5.0, 16.0];
        assert_eq!(v1.dot(v2), 27.0);
    }
}
