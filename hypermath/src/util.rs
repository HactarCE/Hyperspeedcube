//! Common mathematical utility functions that didn't fit anywhere else.

use std::ops::{Add, BitXorAssign, Mul};

use itertools::Itertools;
use num_traits::{CheckedShl, PrimInt, Unsigned};

use crate::Float;

pub const PI: Float = std::f64::consts::PI;

/// Linearly interpolates (unclamped) between two values.
pub fn lerp<A, T>(a: A, b: A, t: T) -> <A::Output as Add>::Output
where
    A: Mul<T>,
    A::Output: Add,
    T: num_traits::Float,
{
    a * (T::one() - t) + b * t
}

/// Linearly interpolates (unclamped) componentwise between two arrays of
/// values.
pub fn lerp_array<A, T, const N: usize>(
    a: [A; N],
    b: [A; N],
    t: T,
) -> [<A::Output as Add>::Output; N]
where
    A: Copy + Mul<T>,
    A::Output: Add,
    T: num_traits::Float,
{
    std::array::from_fn(|i| lerp(a[i], b[i], t))
}

/// Returns the element of an iterator with the minimum value, allowing floats
/// or other `PartialOrd` types.
pub fn min_by_key<T, K: PartialOrd>(
    elems: impl IntoIterator<Item = T>,
    mut f: impl FnMut(&T) -> K,
) -> Option<T> {
    let mut iter = elems.into_iter();
    let mut min_elem = iter.next()?;
    let mut min_key = f(&min_elem);
    for elem in iter {
        let key = f(&elem);
        if key < min_key {
            min_elem = elem;
            min_key = key;
        }
    }
    Some(min_elem)
}

/// Divides `lhs` by `rhs` if the reciprocal of `rhs` is finite; otherwise
/// returns `None`.
pub fn try_div<T>(lhs: T, rhs: Float) -> Option<T::Output>
where
    T: Mul<Float>,
{
    Some(lhs * try_recip(rhs)?)
}

/// Returns the reciprocal of `x` if `x` is nonzero; otherwise returns `None`.
pub fn try_recip(x: Float) -> Option<Float> {
    crate::is_approx_nonzero(&x).then(|| x.recip())
}

/// Returns the square root of `n` if the result is finite; otherwise returns
/// `None`.
pub fn try_sqrt(n: Float) -> Option<Float> {
    let ret = n.sqrt();
    ret.is_finite().then_some(ret)
}

/// Iterator with a manually-specified exact size.
#[derive(Debug, Clone)]
pub struct WithExactSizeIter<I> {
    iter: I,
    len: usize,
}
impl<I: Iterator> Iterator for WithExactSizeIter<I> {
    type Item = I::Item;

    fn next(&mut self) -> Option<Self::Item> {
        if self.len > 0 {
            self.len -= 1;
        }
        self.iter.next()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len, Some(self.len))
    }
}
impl<I: Iterator> ExactSizeIterator for WithExactSizeIter<I> {
    fn len(&self) -> usize {
        self.len
    }
}

/// Extension trait for `.with_exact_size()`.
pub trait IterWithExactSizeExt: Iterator + Sized {
    /// Returns an [`ExactSizeIterator`] that thinks it has length `len`.
    ///
    /// This length is not checked.
    fn with_exact_size(self, len: usize) -> WithExactSizeIter<Self>;
}
impl<I: Iterator> IterWithExactSizeExt for I {
    fn with_exact_size(self, len: usize) -> WithExactSizeIter<Self> {
        WithExactSizeIter { iter: self, len }
    }
}

/// If both options are `Some`, merges them using `f`. Otherwise returns
/// whichever one is `Some`, or `None` if they are both `None`.
pub fn merge_options<T>(a: Option<T>, b: Option<T>, f: impl FnOnce(T, T) -> T) -> Option<T> {
    match (a, b) {
        (Some(a), Some(b)) => Some(f(a, b)),
        (a, b) => a.or(b),
    }
}

/// Iterates over the indices of set bits in `bitset`.
pub fn iter_ones<N: 'static + PrimInt + Unsigned + CheckedShl + BitXorAssign>(
    mut bitset: N,
) -> impl Iterator<Item = u32> {
    std::iter::from_fn(move || {
        let ret = bitset.trailing_zeros();
        bitset ^= N::one().checked_shl(ret)?;
        Some(ret)
    })
}

/// Returns an iterator over the powerset of set bits in `bitset`.
pub fn bitset_powerset<N: 'static + PrimInt + Unsigned + CheckedShl + BitXorAssign>(
    bitset: N,
) -> impl Iterator<Item = N> {
    let set_bits = iter_ones(bitset).collect_vec();
    (0..(1_u32 << bitset.count_ones())).map(move |i| {
        iter_ones(i)
            .map(|j| N::one() << set_bits[j as usize] as usize)
            .fold(N::zero(), |a, b| a + b)
    })
}

/// Zips two iterators together, padding with default values to make their
/// lengths equal.
pub fn pad_zip<A: Default, B: Default>(
    a: impl IntoIterator<Item = A>,
    b: impl IntoIterator<Item = B>,
) -> impl Iterator<Item = (A, B)> {
    a.into_iter()
        .zip_longest(b)
        .map(|either_or_both| match either_or_both {
            itertools::EitherOrBoth::Both(a, b) => (a, b),
            itertools::EitherOrBoth::Left(a) => (a, B::default()),
            itertools::EitherOrBoth::Right(b) => (A::default(), b),
        })
}

/// Returns the minimum and maximum values from an iterator of floats.
pub fn min_max(elems: impl IntoIterator<Item = Float>) -> Option<(Float, Float)> {
    let mut elems = elems.into_iter();
    let mut min = elems.next()?;
    let mut max = min;
    for elem in elems {
        min = Float::min(min, elem);
        max = Float::max(max, elem);
    }
    Some((min, max))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_iter_ones() {
        assert_eq!(iter_ones(0b00000000_u8).collect_vec(), vec![]);
        assert_eq!(iter_ones(0b00000001_u8).collect_vec(), vec![0]);
        assert_eq!(iter_ones(0b00000010_u8).collect_vec(), vec![1]);
        assert_eq!(iter_ones(0b00000011_u8).collect_vec(), vec![0, 1]);
        assert_eq!(iter_ones(0b01010111_u8).collect_vec(), vec![0, 1, 2, 4, 6]);

        assert_eq!(iter_ones(0b10000000_u8).collect_vec(), vec![7]);
        assert_eq!(iter_ones(0b11101010_u8).collect_vec(), vec![1, 3, 5, 6, 7]);
    }

    #[test]
    fn test_bitset_powerset() {
        assert_eq!(
            bitset_powerset(0b11010_u8).collect_vec(),
            vec![0, 2, 8, 10, 16, 18, 24, 26],
        )
    }
}
