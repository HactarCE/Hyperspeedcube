//! Common mathematical utility functions that didn't fit anywhere else.

use std::ops::{Add, Mul};

use itertools::Itertools;
use num_traits::{CheckedShr, PrimInt, Unsigned};

use crate::Float;

/// Linearly interpolates (unclamped) between two values.
pub fn lerp<A, T>(a: A, b: A, t: T) -> <A::Output as Add>::Output
where
    A: Mul<T>,
    A::Output: Add,
    T: num_traits::Float,
{
    a * (T::one() - t) + b * t
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
    let recip_rhs = rhs.recip();
    recip_rhs.is_finite().then(|| lhs * recip_rhs)
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
pub fn iter_ones<
    N: 'static + num_traits::PrimInt + num_traits::Unsigned + num_traits::CheckedShr,
>(
    bitset: N,
) -> impl Iterator<Item = u32> {
    let mut next = bitset.trailing_zeros();
    let bitset = bitset >> 1_usize;
    std::iter::from_fn(move || {
        let ret = next;
        next = next + 1 + bitset.checked_shr(next)?.trailing_zeros();
        Some(ret)
    })
}

/// Returns an iterator over the powerset of set bits in `bitset`.
pub fn bitset_powerset<N: 'static + PrimInt + Unsigned + CheckedShr>(
    bitset: N,
) -> impl Iterator<Item = N> {
    let set_bits = iter_ones(bitset).collect_vec();
    (0..(1_u32 << bitset.count_ones())).map(move |i| {
        iter_ones(i)
            .map(|j| N::one() << set_bits[j as usize] as usize)
            .fold(N::zero(), |a, b| a + b)
    })
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
