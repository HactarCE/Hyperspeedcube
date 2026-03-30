//! Flattened vector type indexed by a newtype.

use std::fmt;
use std::marker::PhantomData;
use std::ops::{Index, IndexMut, Range};

use itertools::Itertools;

use super::{TypedIndex, TypedIndexIter};
use crate::error::{IndexOutOfRange, IndexOverflow};

/// 2D array, where each "row" in the outer array is indexed using a typed index
/// and each "column" in the inner array is indexed using `usize`.
///
/// This is analogous to [`super::TiVec`]`<I, [E; N]>` with `const N: usize`,
/// except that `N` is determined at runtime.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct FlatTiVec<I, E> {
    /// Number of elements `E` in each row.
    column_count: usize,
    /// Number of rows.
    row_count: usize,
    values: Vec<E>,
    _phantom: PhantomData<I>,
}

impl<I: fmt::Debug, E: fmt::Debug> fmt::Debug for FlatTiVec<I, E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let contents = self.values.iter().map(|v| format!("{v:?}")).join(", ");
        write!(f, "[{contents}]")
    }
}

impl<I: fmt::Display, E: fmt::Display> fmt::Display for FlatTiVec<I, E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let contents = self.values.iter().join(", ");
        write!(f, "[{contents}]")
    }
}

impl<I: TypedIndex, E> Index<I> for FlatTiVec<I, E> {
    type Output = [E];

    fn index(&self, index: I) -> &Self::Output {
        let index_range = self.index_range(index);
        &self.values[index_range]
    }
}

impl<I: TypedIndex, E> IndexMut<I> for FlatTiVec<I, E> {
    fn index_mut(&mut self, index: I) -> &mut Self::Output {
        let index_range = self.index_range(index);
        &mut self.values[index_range]
    }
}

impl<I: TypedIndex, E> FlatTiVec<I, E> {
    /// Constructs a new empty vector.
    pub const fn new(column_count: usize) -> Self {
        FlatTiVec {
            column_count,
            row_count: 0,
            values: vec![],
            _phantom: PhantomData,
        }
    }

    /// Constructs a vector from an iterator over rows.
    pub fn from_iter<T: IntoIterator>(column_count: usize, iter: T) -> Self
    where
        T::Item: IntoIterator<Item = E>,
        E: Default,
    {
        let mut len = 0;
        let mut values = vec![];
        for row in iter {
            len += 1;
            values.extend(row_at_len(column_count, row));
        }
        FlatTiVec {
            column_count,
            row_count: len,
            values,
            _phantom: PhantomData,
        }
    }

    /// Constructs a vector with zero columns and an arbitrary number of (empty)
    /// rows.
    pub const fn with_zero_columns(row_count: usize) -> Self {
        Self {
            column_count: 0,
            row_count,
            values: vec![],
            _phantom: PhantomData,
        }
    }

    /// Returns the number of entries in each row.
    pub fn column_count(&self) -> usize {
        self.column_count
    }

    /// Returns the number of rows.
    pub fn row_count(&self) -> usize {
        self.row_count
    }

    /// Returns the range in `self.values` that corresponds to an index.
    ///
    /// Does not return an error if the index is out of bounds. Panics on
    /// overflow.
    fn index_range(&self, index: I) -> Range<usize> {
        let i = index.to_index();
        let start = self.column_count * i;
        start..start + self.column_count
    }

    /// Adds a row of elements to the end of the vector and returns its index.
    ///
    /// The row is truncated or extended using [`Default::default()`] as needed
    /// to fit the row length.
    pub fn push_row(&mut self, values: impl IntoIterator<Item = E>) -> Result<I, IndexOverflow>
    where
        E: Default,
    {
        let idx = self.next_idx()?;
        self.row_count += 1;
        self.values.extend(
            values
                .into_iter()
                .pad_using(self.column_count, |_| E::default())
                .take(self.column_count),
        );
        Ok(idx)
    }
    /// Shorthand for [`Self::push_row`]`(values.into_iter().copied())`.
    pub fn push_row_ref<'a>(
        &mut self,
        values: impl IntoIterator<Item = &'a E>,
    ) -> Result<I, IndexOverflow>
    where
        E: 'a + Copy + Default,
    {
        self.push_row(values.into_iter().copied())
    }

    /// Returns whether there are no rows in the collection is empty.
    ///
    /// A collection is nonempty if there is at least one row, even if there are
    /// no columns.
    pub fn is_empty(&self) -> bool {
        self.row_count == 0
    }
    /// Returns the index of the next row to be added to the collection.
    pub fn next_idx(&self) -> Result<I, IndexOverflow> {
        I::try_from_index(self.row_count)
    }

    /// Returns a reference to the row at `index`, or an error if the index is
    /// out of range.
    pub fn get(&self, index: I) -> Result<&[E], IndexOutOfRange> {
        let index_range = self.index_range(index);
        self.values
            .get(index_range)
            .ok_or(IndexOutOfRange::new::<I>())
    }
    /// Returns a mutable reference to the row at `index`, or an error if the
    /// index is out of range.
    pub fn get_mut(&mut self, index: I) -> Result<&mut [E], IndexOutOfRange> {
        let index_range = self.index_range(index);
        self.values
            .get_mut(index_range)
            .ok_or(IndexOutOfRange::new::<I>())
    }

    /// Returns an iterator over the indexes in the collection.
    pub fn iter_keys(&self) -> TypedIndexIter<I> {
        I::iter(self.row_count)
    }
    /// Returns an iterator over the rows in the collection.
    pub fn iter_rows(&self) -> impl Clone + ExactSizeIterator + DoubleEndedIterator<Item = &[E]> {
        self.iter_keys().map(|i| &self[i])
    }
    /// Returns an iterator over the index-row pairs in the collection.
    pub fn iter(&self) -> Iter<'_, I, E> {
        Iter {
            collection: self,
            indexes: self.iter_keys(),
        }
    }

    /// Returns a reference the underlying flattened slice.
    pub fn as_flattened_slice(&self) -> &[E] {
        &self.values
    }
    /// Returns a mutable reference the underlying flattened slice.
    pub fn as_flattened_slice_mut(&mut self) -> &mut [E] {
        &mut self.values
    }
    /// Converts the collection to a [`Vec`].
    ///
    /// Because [`FlatTiVec<I, E>`] is a newtype wrapper around `Vec<E>`, this
    /// is a no-op.
    pub fn into_flattened_vec(self) -> Vec<E> {
        self.values
    }
}

impl<'a, I: TypedIndex, E> IntoIterator for &'a FlatTiVec<I, E> {
    type Item = (I, &'a [E]);

    type IntoIter = Iter<'a, I, E>;

    fn into_iter(self) -> Self::IntoIter {
        Iter {
            collection: self,
            indexes: self.iter_keys(),
        }
    }
}

/// Borrowing iterator over key-row pairs in a [`FlatTiVec`].
#[derive(Debug)]
pub struct Iter<'a, I, E> {
    collection: &'a FlatTiVec<I, E>,
    indexes: TypedIndexIter<I>,
}

impl<'a, I: Clone, E> Clone for Iter<'a, I, E> {
    fn clone(&self) -> Self {
        Self {
            collection: self.collection,
            indexes: self.indexes.clone(),
        }
    }
}

impl<'a, I: TypedIndex, E> Iterator for Iter<'a, I, E> {
    type Item = (I, &'a [E]);

    fn next(&mut self) -> Option<Self::Item> {
        let i = self.indexes.next()?;
        Some((i, &self.collection[i]))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.indexes.size_hint()
    }
}

impl<I: TypedIndex, E> ExactSizeIterator for Iter<'_, I, E> {}

impl<I: TypedIndex, E> DoubleEndedIterator for Iter<'_, I, E> {
    fn next_back(&mut self) -> Option<Self::Item> {
        let i = self.indexes.next_back()?;
        Some((i, &self.collection[i]))
    }
}

fn row_at_len<E: Default>(
    column_count: usize,
    row: impl IntoIterator<Item = E>,
) -> impl IntoIterator<Item = E> {
    row.into_iter()
        .pad_using(column_count, |_| E::default())
        .take(column_count)
}
