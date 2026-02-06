//! Vector type indexed by a newtype.

use std::fmt;
use std::marker::PhantomData;
use std::ops::{Index, IndexMut};

use itertools::Itertools;

use super::{TypedIndex, TypedIndexIter};
use crate::error::{IndexOutOfRange, IndexOverflow};

/// Wrapper around a `Vec<E>` that is indexed using a typed index. Typed indexes
/// must implement [`TypedIndex`] and can be defined using
/// [`typed_index_struct!`].
#[derive(Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(transparent))]
pub struct TiVec<I, E> {
    values: Vec<E>,
    _phantom: PhantomData<I>,
}

impl<I: fmt::Debug, E: fmt::Debug> fmt::Debug for TiVec<I, E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let contents = self.values.iter().map(|v| format!("{v:?}")).join(", ");
        write!(f, "[{contents}]")
    }
}

impl<I: fmt::Display, E: fmt::Display> fmt::Display for TiVec<I, E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let contents = self.values.iter().join(", ");
        write!(f, "[{contents}]")
    }
}

impl<I, E> Default for TiVec<I, E> {
    fn default() -> Self {
        Self {
            values: vec![],
            _phantom: PhantomData,
        }
    }
}

impl<I: TypedIndex, E> Index<I> for TiVec<I, E> {
    type Output = E;

    fn index(&self, index: I) -> &Self::Output {
        &self.values[index.to_u64() as usize]
    }
}

impl<I: TypedIndex, E> IndexMut<I> for TiVec<I, E> {
    fn index_mut(&mut self, index: I) -> &mut Self::Output {
        &mut self.values[index.to_u64() as usize]
    }
}

impl<I, E> std::ops::Deref for TiVec<I, E> {
    type Target = [E];

    fn deref(&self) -> &Self::Target {
        &self.values
    }
}

impl<I, E> std::ops::DerefMut for TiVec<I, E> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.values
    }
}

impl<I: TypedIndex, E> TiVec<I, E> {
    /// Constructs a new empty vector.
    pub const fn new() -> Self {
        TiVec {
            values: vec![],
            _phantom: PhantomData,
        }
    }
    /// Constructs a new vector with the given length, filling it with default
    /// values.
    pub fn new_with_len(len: usize) -> Self
    where
        E: Default,
    {
        I::iter(len).map(|_| E::default()).collect()
    }

    /// Adds an element to the end of the vector and returns its index.
    pub fn push(&mut self, value: E) -> Result<I, IndexOverflow> {
        let idx = self.next_idx()?;
        self.values.push(value);
        Ok(idx)
    }

    /// Extends the vector until it contains `index`.
    pub fn extend_to_contain(&mut self, index: I)
    where
        E: Default,
    {
        while index.to_u64() >= self.len() as u64 {
            self.push(E::default()).expect("impossible overflow!");
        }
    }
    /// Resizes the vector to exactly `len`.
    pub fn resize(&mut self, len: usize) -> Result<(), IndexOverflow>
    where
        E: Default,
    {
        self.resize_with(len, E::default)
    }
    /// Resizes the vector to exactly `len`, using `f` to generate new elements
    /// as needed.
    pub fn resize_with(&mut self, len: usize, f: impl FnMut() -> E) -> Result<(), IndexOverflow> {
        // Check that the new length is valid.
        I::try_from_index(len.saturating_sub(1))?;

        self.values.resize_with(len, f);
        Ok(())
    }

    /// Returns whether the collection is empty.
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }
    /// Returns the number of elements in the collection.
    pub fn len(&self) -> usize {
        self.values.len()
    }
    /// Returns the index of the next element to be added to the collection.
    pub fn next_idx(&self) -> Result<I, IndexOverflow> {
        I::try_from_index(self.len())
    }

    /// Returns a reference to the element at `index`, or an error if the index
    /// is out of range.
    pub fn get(&self, index: I) -> Result<&E, IndexOutOfRange> {
        self.values.get(index.to_index()).ok_or(IndexOutOfRange {
            type_name: I::TYPE_NAME,
        })
    }
    /// Returns a mutable reference to the element at `index`, or an error if
    /// the index is out of range.
    pub fn get_mut(&mut self, index: I) -> Result<&mut E, IndexOutOfRange> {
        self.values
            .get_mut(index.to_index())
            .ok_or(IndexOutOfRange {
                type_name: I::TYPE_NAME,
            })
    }

    /// Swaps two elements, or returns an error if the index is out of range.
    pub fn swap(&mut self, i: I, j: I) -> Result<(), IndexOutOfRange> {
        let i = i.to_index();
        let j = j.to_index();
        if i < self.len() && j < self.len() {
            self.values.swap(i, j);
            Ok(())
        } else {
            Err(IndexOutOfRange {
                type_name: I::TYPE_NAME,
            })
        }
    }

    /// Returns an iterator over the indexes in the collection.
    pub fn iter_keys(&self) -> TypedIndexIter<I> {
        I::iter(self.len())
    }
    /// Returns an iterator over the values in the collection.
    pub fn iter_values(&self) -> impl DoubleEndedIterator<Item = &E> {
        self.values.iter()
    }
    /// Returns a mutating iterator over the values in the collections.
    pub fn iter_values_mut(&mut self) -> impl DoubleEndedIterator<Item = &mut E> {
        self.values.iter_mut()
    }
    /// Returns an iterator over the index-value pairs in the collection.
    pub fn iter(&self) -> impl DoubleEndedIterator<Item = (I, &E)> {
        self.iter_keys().zip(&self.values)
    }
    /// Returns a mutating iterator over the index-value pairs in the
    /// collection.
    pub fn iter_mut(&mut self) -> impl DoubleEndedIterator<Item = (I, &mut E)> {
        self.iter_keys().zip(&mut self.values)
    }

    /// Returns an iterator over keys for which a predicate returns `true`.
    pub fn iter_filter<'a>(
        &'a self,
        mut pred: impl 'a + FnMut(I, &E) -> bool,
    ) -> impl 'a + DoubleEndedIterator<Item = I> {
        self.iter_keys().filter(move |&i| pred(i, &self[i]))
    }
    /// Returns the first key for which a predicate returns `true`.
    pub fn find(&self, pred: impl FnMut(I, &E) -> bool) -> Option<I> {
        self.iter_filter(pred).next()
    }
    /// Returns the last key for which a predicate returns `true`.
    pub fn rfind(&self, pred: impl FnMut(I, &E) -> bool) -> Option<I> {
        self.iter_filter(pred).next_back()
    }

    /// Applies a function to every value in the collection and returns a new
    /// collection.
    pub fn map<U>(self, mut f: impl FnMut(I, E) -> U) -> TiVec<I, U> {
        self.into_iter().map(|(i, e)| f(i, e)).collect()
    }
    /// Applies a function to every value in the collection and returns a new
    /// collection.
    pub fn map_ref<'a, U>(&'a self, mut f: impl FnMut(I, &'a E) -> U) -> TiVec<I, U> {
        self.iter().map(|(i, e)| f(i, e)).collect()
    }
    /// Applies a function to every value in the collection and returns a new
    /// collection, or the first error returned by the function.
    pub fn try_map<U, S>(self, mut f: impl FnMut(I, E) -> Result<U, S>) -> Result<TiVec<I, U>, S> {
        Ok(self
            .into_iter()
            .map(|(i, e)| f(i, e))
            .collect::<Result<Vec<_>, _>>()?
            .into())
    }
    /// Applies a function to every value in the collection and returns a new
    /// collection, or the first error returned by the function.
    pub fn try_map_ref<'a, U, S>(
        &'a self,
        mut f: impl FnMut(I, &'a E) -> Result<U, S>,
    ) -> Result<TiVec<I, U>, S> {
        Ok(self
            .iter()
            .map(|(i, e)| f(i, e))
            .collect::<Result<Vec<_>, _>>()?
            .into())
    }
}

impl<I: TypedIndex, E> TiVec<I, Option<E>> {
    /// Returns a reference to the element at `index`, collapsing
    /// `Result<Option<E>>` to `E`.
    ///
    /// Short for `self.get(index).ok().and_then(Option::as_ref)`.
    pub fn get_opt(&self, index: I) -> Option<&E> {
        self.get(index).ok().and_then(Option::as_ref)
    }
}

impl<I: TypedIndex, E> FromIterator<E> for TiVec<I, E> {
    fn from_iter<T: IntoIterator<Item = E>>(iter: T) -> Self {
        let values = iter
            .into_iter()
            .take(I::MAX_INDEX.saturating_add(1))
            .collect_vec();
        TiVec {
            values,
            _phantom: PhantomData,
        }
    }
}

impl<I: TypedIndex, E> From<Vec<E>> for TiVec<I, E> {
    fn from(mut values: Vec<E>) -> Self {
        values.truncate(I::MAX_INDEX.saturating_add(1));
        TiVec {
            values,
            _phantom: PhantomData,
        }
    }
}

impl<I: TypedIndex, E> IntoIterator for TiVec<I, E> {
    type Item = (I, E);

    type IntoIter = IntoIter<I, E>;

    fn into_iter(self) -> Self::IntoIter {
        IntoIter {
            indexes: self.iter_keys(),
            values: self.values.into_iter(),
        }
    }
}

/// Owning iterator over key-value pairs in a [`TiVec`].
pub struct IntoIter<I, E> {
    indexes: TypedIndexIter<I>,
    values: std::vec::IntoIter<E>,
}

impl<I: TypedIndex, E> Iterator for IntoIter<I, E> {
    type Item = (I, E);

    fn next(&mut self) -> Option<Self::Item> {
        Some((self.indexes.next()?, self.values.next()?))
    }
}

impl<'a, I: TypedIndex, E> IntoIterator for &'a TiVec<I, E> {
    type Item = (I, &'a E);

    type IntoIter = Iter<'a, I, E>;

    fn into_iter(self) -> Self::IntoIter {
        Iter {
            indexes: self.iter_keys(),
            values: self.values.iter(),
        }
    }
}
/// Borrowing iterator over key-value pairs in a [`TiVec`].
pub struct Iter<'a, I, E> {
    indexes: TypedIndexIter<I>,
    values: std::slice::Iter<'a, E>,
}

impl<'a, I: TypedIndex, E> Iterator for Iter<'a, I, E> {
    type Item = (I, &'a E);

    fn next(&mut self) -> Option<Self::Item> {
        Some((self.indexes.next()?, self.values.next()?))
    }
}

impl<'a, I: TypedIndex, E> IntoIterator for &'a mut TiVec<I, E> {
    type Item = (I, &'a mut E);

    type IntoIter = IterMut<'a, I, E>;

    fn into_iter(self) -> Self::IntoIter {
        IterMut {
            indexes: self.iter_keys(),
            values: self.values.iter_mut(),
        }
    }
}
/// Mutably borrowing iterator over key-value pairs in a [`TiVec`].
pub struct IterMut<'a, I, E> {
    indexes: TypedIndexIter<I>,
    values: std::slice::IterMut<'a, E>,
}

impl<'a, I: TypedIndex, E> Iterator for IterMut<'a, I, E> {
    type Item = (I, &'a mut E);

    fn next(&mut self) -> Option<Self::Item> {
        Some((self.indexes.next()?, self.values.next()?))
    }
}
