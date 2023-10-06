//! Common mathematical utility functions that didn't fit anywhere else.

use std::ops::{Add, Mul};

use super::Float;

/// Linearly interpolates (unclamped) between two values.
pub fn mix<T>(a: T, b: T, t: Float) -> <T::Output as Add>::Output
where
    T: Mul<Float>,
    T::Output: Add,
{
    a * (1.0 - t) + b * t
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
