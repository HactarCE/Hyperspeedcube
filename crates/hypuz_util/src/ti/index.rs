use std::fmt;
use std::marker::PhantomData;
use std::ops::Range;

use crate::error::IndexOverflow;

/// Typed index.
///
/// This is typically a wrapper around a primitive unsigned integer. Integers
/// larger than `usize` are allowed but not all values may be supported.
///
/// Instead of implementing this trait manually, consider using the macro
/// [`typed_index_struct!`].
pub trait TypedIndex:
    'static
    + fmt::Debug
    + fmt::Display
    + Default
    + Copy
    + Clone
    + PartialEq
    + Eq
    + std::hash::Hash
    + PartialOrd
    + Ord
    + tinyset::Fits64
    + Send
    + Sync
{
    /// Maximum value for the type.
    const MAX: Self;
    /// Maximum index representable by the type.
    const MAX_INDEX: usize;
    /// User-friendly type name (lowercase).
    const TYPE_NAME: &'static str;

    /// Returns the index as a `usize`.
    fn to_index(self) -> usize;

    /// Returns an index from a `usize`, or an error if it does not fit.
    fn try_from_index(index: usize) -> Result<Self, IndexOverflow>;

    /// Returns an iterator over all indexes up to `count` (exclusive). If
    /// `count` exceeds the maximum value, then the iterator stops before
    /// reaching the maximum value.
    fn iter(count: usize) -> TypedIndexIter<Self> {
        // Clip to `Self::MAX`
        let count = std::cmp::min(count, Self::MAX_INDEX + 1);
        TypedIndexIter {
            range: 0..count,
            _phantom: PhantomData,
        }
    }

    /// Increments the index, or returns an error if it does not fit.
    fn next(self) -> Result<Self, IndexOverflow> {
        Self::try_from_index(self.to_index().saturating_add(1))
    }

    /// Increments the index in-place and returns the old one, or returns an
    /// error if it doesn't fit.
    fn take_and_increment(&mut self) -> Result<Self, IndexOverflow> {
        Ok(std::mem::replace(self, self.next()?))
    }
}

/// Iterator over all indexes up to a certain value. See [`TypedIndex::iter()`].
#[derive(Debug, Default, Clone)]
pub struct TypedIndexIter<I> {
    range: Range<usize>,
    _phantom: PhantomData<fn() -> I>,
}

impl<I: TypedIndex> Iterator for TypedIndexIter<I> {
    type Item = I;

    fn next(&mut self) -> Option<Self::Item> {
        self.range.next().map(unwrap_index)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.range.size_hint()
    }
}

impl<I: TypedIndex> DoubleEndedIterator for TypedIndexIter<I> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.range.next_back().map(unwrap_index)
    }
}

impl<I: TypedIndex> ExactSizeIterator for TypedIndexIter<I> {}

fn unwrap_index<I: TypedIndex>(index: usize) -> I {
    I::try_from_index(index).expect("error constructing typed index from usize")
}
