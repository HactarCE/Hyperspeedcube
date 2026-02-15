//! N-dimensional vector math.

use std::fmt;
use std::hash::Hash;
use std::iter::Sum;
use std::ops::*;

use approx_collections::{ApproxEq, ApproxEqZero, ApproxHash, ApproxInternable};
use itertools::Itertools;
use smallvec::SmallVec;

use crate::{APPROX, Float, Ndim, util};

/// Constructs an N-dimensional vector, using the same syntax as `vec![]`.
#[macro_export]
macro_rules! vector {
    [$($tok:tt)*] => {
        $crate::Vector($crate::smallvec::smallvec![$($tok)*])
    };
}

/// N-dimensional vector. Indexing out of bounds returns zero.
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(transparent)
)]
#[derive(Debug, Default, Clone, PartialEq)]
pub struct Vector(pub SmallVec<[Float; 4]>);

/// Reference to an N-dimensional vector. Indexing out of bounds returns zero.
pub trait VectorRef: Sized + fmt::Debug + ApproxEq + ApproxEqZero + Ndim {
    /// Converts the vector to a `Vector`.
    fn to_vector(&self) -> Vector {
        self.iter().collect()
    }

    /// Returns a component of the vector. If the index is out of bounds,
    /// returns zero.
    fn get(&self, idx: u8) -> Float;

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
    /// Returns an iterator over the nonzero components of the vector along with
    /// their axes.
    fn iter_nonzero(&self) -> VectorIterNonzero<&Self> {
        VectorIterNonzero {
            range: 0..self.ndim(),
            vector: self,
        }
    }

    /// Returns the dot product of this vector with another.
    fn dot(&self, rhs: impl VectorRef) -> Float {
        // Don't use `Vector::zip()` because that will include zeros at the end
        // that we don't need.
        std::iter::zip(self.iter(), rhs.iter())
            .map(|(l, r)| l * r)
            .sum()
    }
    /// Returns the cross product of two vectors in 3D. Components besides XYZ
    /// are ignored.
    fn cross_product_3d(&self, rhs: impl VectorRef) -> Vector {
        vector![
            self.get(1) * rhs.get(2) - self.get(2) * rhs.get(1),
            self.get(2) * rhs.get(0) - self.get(0) * rhs.get(2),
            self.get(0) * rhs.get(1) - self.get(1) * rhs.get(0),
        ]
    }

    /// Pads the vector with zeros up to `ndim`.
    #[must_use]
    fn pad(&self, ndim: u8) -> Vector {
        self.iter().pad_using(ndim as usize, |_| 0.0).collect()
    }

    /// Returns the magnitude of the vector.
    fn mag(&self) -> Float {
        self.mag2().sqrt()
    }
    /// Returns the squared magnitude of the vector.
    fn mag2(&self) -> Float {
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
    fn scale(&self, scalar: Float) -> Vector {
        self.iter().map(|x| x * scalar).collect()
    }

    /// Returns the component of the vector that is parallel to `other`.
    ///
    /// Returns `None` if `other` is zero.
    fn projected_to(&self, other: &Vector) -> Option<Vector> {
        let scale_factor = util::try_div(self.dot(other), other.mag2())?;
        Some(other * scale_factor)
    }
    /// Returns the component of the vector that is perpendicular to `other`.
    ///
    /// Returns `None` if `other` is zero.
    fn rejected_from(&self, other: &Vector) -> Option<Vector> {
        Some(-self.projected_to(other)? + self)
    }
}

/// Iterator over the nonzero components of a vector.
pub struct VectorIterNonzero<V> {
    range: Range<u8>,
    vector: V,
}
impl<V: VectorRef> Iterator for VectorIterNonzero<V> {
    type Item = (u8, Float);

    fn next(&mut self) -> Option<Self::Item> {
        self.range.find_map(|i| {
            let x = self.vector.get(i);
            APPROX.ne_zero(x).then_some((i, x))
        })
    }
}

/// Iterator over the components of a vector.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct VectorIter<V> {
    range: Range<u8>,
    vector: V,
}
impl<V: VectorRef> Iterator for VectorIter<V> {
    type Item = Float;

    fn next(&mut self) -> Option<Self::Item> {
        self.range.next().map(|i| self.vector.get(i))
    }
}

impl Ndim for Vector {
    /// Returns the number of components in the vector.
    fn ndim(&self) -> u8 {
        self.0.len() as _
    }
}
impl VectorRef for Vector {
    fn get(&self, idx: u8) -> Float {
        self.0.get(idx as usize).copied().unwrap_or(0.0)
    }
}

impl Ndim for &[Float] {
    /// Returns the number of components in the vector.
    fn ndim(&self) -> u8 {
        self.len().try_into().unwrap_or(u8::MAX)
    }
}
impl VectorRef for &[Float] {
    fn get(&self, idx: u8) -> Float {
        <[Float]>::get(self, idx as usize).copied().unwrap_or(0.0)
    }
}

impl<const N: usize> Ndim for [Float; N] {
    /// Returns the number of components in the vector.
    fn ndim(&self) -> u8 {
        self.len().try_into().unwrap_or(u8::MAX)
    }
}
impl<const N: usize> VectorRef for [Float; N] {
    fn get(&self, idx: u8) -> Float {
        <[Float]>::get(self, idx as usize).copied().unwrap_or(0.0)
    }
}

impl<V: VectorRef> Ndim for &'_ V {
    /// Returns the number of components in the vector.
    fn ndim(&self) -> u8 {
        (*self).ndim()
    }
}
impl<V: VectorRef> VectorRef for &'_ V {
    fn get(&self, idx: u8) -> Float {
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

macro_rules! impl_zero_padded_op {
    (impl $trait_name:ident for $type_name:ty { fn $fn_name:ident() }) => {
        impl<V: VectorRef> $trait_name<V> for $type_name {
            type Output = Vector;

            fn $fn_name(self, rhs: V) -> Self::Output {
                Vector::zip(self, rhs).map(|(l, r)| l.$fn_name(r)).collect()
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

        impl Mul<Float> for $type_name {
            type Output = Vector;

            fn mul(self, rhs: Float) -> Self::Output {
                self.iter().map(|x| x * rhs).collect()
            }
        }
        impl Div<Float> for $type_name {
            type Output = Vector;

            #[allow(clippy::suspicious_arithmetic_impl)]
            fn div(self, rhs: Float) -> Self::Output {
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
impl<V: VectorRef> SubAssign<V> for Vector {
    fn sub_assign(&mut self, rhs: V) {
        let ndim = std::cmp::max(self.ndim(), rhs.ndim());
        self.0.resize(ndim as _, 0.0);
        for i in 0..rhs.ndim() {
            self[i] -= rhs.get(i);
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
    type Output = Float;

    fn index(&self, index: u8) -> &Self::Output {
        &self.0[index as usize]
    }
}
impl Index<u8> for &'_ Vector {
    type Output = Float;

    fn index(&self, index: u8) -> &Self::Output {
        &self.0[index as usize]
    }
}
impl IndexMut<u8> for Vector {
    fn index_mut(&mut self, index: u8) -> &mut Self::Output {
        let ndim = self.ndim();
        self.0.get_mut(index as usize).unwrap_or_else(|| {
            panic!(
                "vector index out of bounds: the dimensionality is {ndim} but the index is {index}",
            )
        })
    }
}

impl Vector {
    /// Zero-dimensional empty vector.
    pub const EMPTY: Self = Self(SmallVec::new_const());

    /// Returns a zero vector.
    pub fn zero(ndim: u8) -> Self {
        let mut ret = Self::EMPTY;
        ret.resize(ndim);
        ret
    }
    /// Returns a unit vector along an axis.
    pub fn unit(axis: u8) -> Self {
        let mut ret = vector![0.0; axis as usize + 1];
        ret[axis] = 1.0;
        ret
    }

    /// Resizes the vector in-place, padding with zeros.
    pub fn resize(&mut self, ndim: u8) {
        self.0.resize(ndim as _, 0.0);
    }
    /// Sets a value in the vector, resizing the vector by padding with zeros if
    /// it is not already long enough.
    pub fn resize_and_set(&mut self, axis: u8, value: Float) {
        if axis >= self.ndim() {
            self.resize(axis + 1);
        }
        self[axis] = value;
    }
    /// Resizes the vector in-place to the minimum dimension with the same
    /// value.
    ///
    /// This uses exact equality, _not_ approximate equality.
    fn resize_to_min_ndim(&mut self) {
        let new_len = self.0.len() - self.0.iter().rev().take_while(|&&x| x == 0.0).count();
        self.0.truncate(new_len);
    }

    /// Returns an iterator over two vectors, both padded to the same length.
    pub fn zip<A: VectorRef, B: VectorRef>(
        a: A,
        b: B,
    ) -> std::iter::Zip<VectorIter<A>, VectorIter<B>> {
        let max_ndim = std::cmp::max(a.ndim(), b.ndim());
        std::iter::zip(
            VectorIter {
                range: 0..max_ndim,
                vector: a,
            },
            VectorIter {
                range: 0..max_ndim,
                vector: b,
            },
        )
    }
}

macro_rules! impl_vector_approx_eq {
    (impl for $type:ty) => {
        impl approx_collections::ApproxEq for $type {
            fn approx_eq(&self, other: &Self, prec: approx_collections::Precision) -> bool {
                $crate::Vector::zip(self, other).all(|(l, r)| prec.eq(l, r))
            }
        }

        impl approx_collections::ApproxEqZero for $type {
            fn approx_eq_zero(&self, prec: approx_collections::Precision) -> bool {
                self.iter().all(|x| prec.eq_zero(x))
            }
        }
    };
}

impl_vector_approx_eq!(impl for Vector);

impl ApproxInternable for Vector {
    fn intern_floats<F: FnMut(&mut f64)>(&mut self, f: &mut F) {
        self.0.intern_floats(f);
        self.resize_to_min_ndim();
    }
}
impl ApproxHash for Vector {
    fn interned_eq(&self, other: &Self) -> bool {
        Vector::zip(self, other).all(|(a, b)| a.interned_eq(&b))
    }

    fn interned_hash<H: std::hash::Hasher>(&self, state: &mut H) {
        for (i, x) in self.iter().enumerate() {
            if !x.interned_eq(&0.0) {
                i.hash(state);
                x.interned_hash(state);
            }
        }
    }
}

impl FromIterator<Float> for Vector {
    fn from_iter<T: IntoIterator<Item = Float>>(iter: T) -> Self {
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
