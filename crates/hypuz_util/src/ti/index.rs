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
    + Send
    + Sync
{
    /// Maximum value for the type.
    const MAX: Self;
    /// Maximum index representable by the type.
    ///
    /// This **must not** be equal to [`usize::MAX`]. It may equal
    /// `usize::MAX-1`.
    const MAX_INDEX: usize;
    /// User-friendly type name (lowercase).
    const TYPE_NAME: &'static str;

    /// Returns the index as a `usize`.
    fn to_index(self) -> usize;

    /// Returns an index from a `usize`, or an error if it does not fit.
    fn try_from_index(index: usize) -> Result<Self, IndexOverflow>;

    /// Returns an iterator over all indexes up to `count` (exclusive).
    ///
    /// # Panics
    ///
    /// Panics if `count-1 > `[`Self::MAX_INDEX`].
    #[track_caller]
    fn iter(count: usize) -> TypedIndexIter<Self> {
        Self::iter_range(0..count)
    }

    /// Returns an iterator over all indexes up to `count` (exclusive).
    ///
    /// Returns an error if `count-1 > `[`Self::MAX_INDEX`].
    fn try_iter(count: usize) -> Result<TypedIndexIter<Self>, IndexOverflow> {
        Self::try_iter_range(0..count)
    }

    /// Returns an iterator over all indexes up to `count` (exclusive).
    ///
    /// The iterator never yields elements past the maximum value, even if
    /// `count` exceeds the maximum value.
    fn iter_clamped(count: usize) -> TypedIndexIter<Self> {
        Self::iter_range_clamped(0..count)
    }

    /// Returns an iterator over all indexes in `range`.
    ///
    /// # Panics
    ///
    /// Panics if `range.end-1 > `[`Self::MAX_INDEX`].
    #[track_caller]
    fn iter_range(range: Range<usize>) -> TypedIndexIter<Self> {
        #[allow(clippy::unwrap_used)] // IndexOverflow gives good error message
        Self::try_iter_range(range).unwrap()
    }

    /// Returns an iterator over all indexes in `range`.
    ///
    /// Returns an error if `range.end-1 > `[`Self::MAX_INDEX`].
    fn try_iter_range(range: Range<usize>) -> Result<TypedIndexIter<Self>, IndexOverflow> {
        if range.end > Self::MAX_INDEX + 1 {
            return Err(IndexOverflow::new::<Self>());
        }
        Ok(TypedIndexIter {
            range,
            _phantom: PhantomData,
        })
    }

    /// Returns an iterator over all indexes in `range`.
    ///
    /// The iterator never yields elements past the maximum value, even if
    /// `range.end` exceeds the maximum value.
    fn iter_range_clamped(mut range: Range<usize>) -> TypedIndexIter<Self> {
        if range.end > Self::MAX_INDEX + 1 {
            range.end = Self::MAX_INDEX + 1;
        }
        TypedIndexIter {
            range,
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

/// Iterator over all indexes in a range. See [`TypedIndex::iter()`].
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

    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        self.range.nth(n).map(unwrap_index)
    }

    fn count(self) -> usize
    where
        Self: Sized,
    {
        self.range.count()
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

impl<I: TypedIndex> TypedIndexIter<I> {
    /// Returns the same range as `Range<usize>`.
    pub fn to_usize_range(&self) -> Range<usize> {
        self.range.clone()
    }
}

fn unwrap_index<I: TypedIndex>(index: usize) -> I {
    I::try_from_index(index).expect("error constructing typed index from usize")
}
