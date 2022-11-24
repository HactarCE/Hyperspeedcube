//! Common utility functions.

use tinyset::Set64;

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
    /// Returns an `ExactSizeIterator` that thinks it has length `len`.
    ///
    /// This length is not checked.
    fn with_exact_size(self, len: usize) -> WithExactSizeIter<Self>;
}
impl<I: Iterator> IterWithExactSizeExt for I {
    fn with_exact_size(self, len: usize) -> WithExactSizeIter<Self> {
        WithExactSizeIter { iter: self, len }
    }
}

/// Stolen from
/// https://github.com/rust-lang/rust/blob/e6ce5627a9e8af9ae4673a390954fffaf526e5cc/library/core/src/num/int_macros.rs#L2204-L2222
///
/// When #![feature(int_roundings)] is merged, delete this.
pub fn next_multiple_of(lhs: u64, rhs: u64) -> u64 {
    let m = lhs % rhs;

    if m == 0 {
        lhs
    } else {
        lhs + (rhs - m)
    }
}

/// Extension trait for `.fold_intersection()`.
pub trait Set64IntersectionIterExt {
    /// Output of `.fold_intersection()`.
    type Output;

    /// Returns the intersection of all the sets produced by an iterator.
    fn fold_intersection(self) -> Self::Output;
}
impl<'a, I, T: 'a> Set64IntersectionIterExt for I
where
    I: Iterator<Item = &'a Set64<T>>,
    T: tinyset::Fits64,
{
    type Output = Set64<T>;

    fn fold_intersection(mut self) -> Set64<T> {
        let mut ret = self.next().unwrap_or(&Set64::new()).clone();
        for it in self {
            ret = set64_intersection(&ret, it);
        }
        ret
    }
}
/// Extension trait for `.try_fold_intersection()`.
pub trait Set64TryIntersectionIterExt {
    /// Output of `.try_fold_intersection()`.
    type Output;

    /// Returns the intersection of all the sets produced by an iterator, or an
    /// error if any element is `Err`.
    fn try_fold_intersection(self) -> Self::Output;
}
impl<'a, I, T: 'a, E> Set64TryIntersectionIterExt for I
where
    I: Iterator<Item = Result<&'a Set64<T>, E>>,
    T: tinyset::Fits64,
{
    type Output = Result<Set64<T>, E>;

    fn try_fold_intersection(mut self) -> Result<Set64<T>, E> {
        let mut ret = self.next().unwrap_or(Ok(&Set64::new()))?.clone();
        for it in self {
            ret = set64_intersection(&ret, it?);
        }
        Ok(ret)
    }
}

/// Returns whether `a` is a subset of `b`.
pub fn is_subset<T: tinyset::Fits64>(a: &Set64<T>, b: &Set64<T>) -> bool {
    a.iter().all(|elem| b.contains(elem))
}

/// Returns the intersection of `a` and `b`.
pub fn set64_intersection<T: tinyset::Fits64>(a: &Set64<T>, b: &Set64<T>) -> Set64<T> {
    a.iter().filter(|elem| b.contains(elem)).collect()
}
