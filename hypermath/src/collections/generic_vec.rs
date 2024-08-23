//! Vector type indexed by a newtype.

use std::fmt;
use std::marker::PhantomData;
use std::ops::{Index, IndexMut, Range};

use itertools::Itertools;

/// Error value returned by some operations related to [`GenericVec`]s when the
/// maximum value of an indexing type is exceeded.
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct IndexOverflow {
    /// Name of the indexing type.
    pub type_name: &'static str,
    /// Maximum allowed value for the indexing type.
    pub max_value: u64,
}
impl fmt::Display for IndexOverflow {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "exceeded maximum {} count of {}",
            self.type_name, self.max_value,
        )
    }
}
impl std::error::Error for IndexOverflow {}

/// Error value returned by some operations related to [`GenericVec`]s when the
/// index is too large for the vector.
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct IndexOutOfRange {
    /// Name of the indexing type.
    pub type_name: &'static str,
}
impl fmt::Display for IndexOutOfRange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} index out of range", self.type_name)
    }
}
impl std::error::Error for IndexOutOfRange {}

/// Constructs a struct that is a simple wrapper around a primitive unsigned
/// integer type used as an index.
#[macro_export]
macro_rules! idx_struct {
    (
        $(
            $(#[$attr:meta])*
            $struct_vis:vis struct $struct_name:ident($inner_vis:vis $inner_type:ty);
        )+
    ) => {
        $(
            $(#[$attr])*
            #[cfg_attr(feature = "bytemuck", derive(bytemuck::Pod, bytemuck::Zeroable))]
            #[derive(Default, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
            #[repr(transparent)]
            $struct_vis struct $struct_name($inner_vis $inner_type);

            impl ::std::fmt::Debug for $struct_name {
                fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                    write!(f, "#{:?}", self.0)
                }
            }
            impl ::std::fmt::Display for $struct_name {
                fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                    write!(f, "#{}", self.0 + 1) // 1-indexing in Lua
                }
            }

            impl $crate::Fits64 for $struct_name {
                unsafe fn from_u64(x: u64) -> Self {
                    Self(x as _)
                }

                fn to_u64(self) -> u64 {
                    self.0 as u64
                }
            }

            impl $crate::collections::generic_vec::IndexNewtype for $struct_name {
                const MAX: Self = Self(<$inner_type>::MAX);
                const MAX_INDEX: usize = <$inner_type>::MAX as usize;
                const TYPE_NAME: &'static str = stringify!($struct_name);

                fn try_from_usize(index: usize) -> Result<Self, $crate::collections::generic_vec::IndexOverflow> {
                    match index.try_into() {
                        Ok(i) => Ok(Self(i)),
                        Err(_) => Err($crate::collections::generic_vec::IndexOverflow {
                            type_name: stringify!($struct_name),
                            max_value: <$inner_type>::MAX as u64,
                        }),
                    }
                }
            }
        )+
    };
}

/// Newtype wrapper around a primitive unsigned integer, which is useful as an
/// index into arrays.
pub trait IndexNewtype:
    fmt::Debug
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
    /// Maximum index representable by the type.
    const MAX: Self;
    /// Maximum index representable by the type.
    const MAX_INDEX: usize;
    /// User-friendly type name (lowercase).
    const TYPE_NAME: &'static str;

    /// Returns the index as a `usize`.
    fn to_usize(self) -> usize {
        self.to_u64() as usize
    }

    /// Returns an index from a `usize`, or an error if it does not fit.
    fn try_from_usize(index: usize) -> Result<Self, IndexOverflow>;

    /// Returns an iterator over all indices up to `count` (exclusive). If
    /// `count` exceeds the maximum value, then the iterator stops before
    /// reaching the maximum value.
    fn iter(count: usize) -> IndexIter<Self> {
        // Clip to `Self::MAX`
        let count = std::cmp::min(count, Self::MAX_INDEX.saturating_add(1));
        IndexIter {
            range: 0..count,
            _phantom: PhantomData,
        }
    }

    /// Increments the index, or returns an error if it does not fit.
    fn next(self) -> Result<Self, IndexOverflow> {
        Self::try_from_usize(self.to_usize().checked_add(1).unwrap_or(usize::MAX))
    }
    /// Increments the index in-place and returns the old one, or returns an
    /// error if it doesn't fit.
    fn take_and_increment(&mut self) -> Result<Self, IndexOverflow> {
        Ok(std::mem::replace(self, self.next()?))
    }
}

/// Iterator over possible indices into a [`GenericVec<I, _>`].
#[derive(Debug, Default, Clone)]
pub struct IndexIter<I> {
    range: Range<usize>,
    _phantom: PhantomData<I>,
}
impl<I: IndexNewtype> Iterator for IndexIter<I> {
    type Item = I;

    fn next(&mut self) -> Option<Self::Item> {
        // SAFETY: This `unsafe` is sound because `IndexIter` is only
        // constructed from a `GenericVec`, and every time a `GenericVec` is
        // constructed or extended we panic if its length exceeds
        // `I::MAX_INDEX`.
        self.range.next().map(|i| unsafe { I::from_u64(i as u64) })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.range.size_hint()
    }
}
impl<I: IndexNewtype> DoubleEndedIterator for IndexIter<I> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.range.next_back().map(|i| {
            // SAFETY: see `next()` above.
            unsafe { I::from_u64(i as u64) }
        })
    }
}
impl<I: IndexNewtype> ExactSizeIterator for IndexIter<I> {}

/// Wrapper around a `Vec<E>` that is indexed using `I` by converting it to an
/// integer.
///
/// Elements are stored using indices.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct GenericVec<I, E> {
    values: Vec<E>,
    _phantom: PhantomData<I>,
}
impl<I: fmt::Debug, E: fmt::Debug> fmt::Debug for GenericVec<I, E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let contents = self.values.iter().map(|v| format!("{v:?}")).join(", ");
        write!(f, "[{contents}]")
    }
}
impl<I: fmt::Display, E: fmt::Display> fmt::Display for GenericVec<I, E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let contents = self.values.iter().join(", ");
        write!(f, "[{contents}]")
    }
}
impl<I, E> Default for GenericVec<I, E> {
    fn default() -> Self {
        Self {
            values: vec![],
            _phantom: PhantomData,
        }
    }
}
impl<I: IndexNewtype, E> Index<I> for GenericVec<I, E> {
    type Output = E;

    fn index(&self, index: I) -> &Self::Output {
        &self.values[index.to_u64() as usize]
    }
}
impl<I: IndexNewtype, E> IndexMut<I> for GenericVec<I, E> {
    fn index_mut(&mut self, index: I) -> &mut Self::Output {
        &mut self.values[index.to_u64() as usize]
    }
}
impl<I, E> std::ops::Deref for GenericVec<I, E> {
    type Target = [E];

    fn deref(&self) -> &Self::Target {
        &self.values
    }
}
impl<I, E> std::ops::DerefMut for GenericVec<I, E> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.values
    }
}
impl<I: IndexNewtype, E> GenericVec<I, E> {
    /// Constructs a new empty slab.
    pub const fn new() -> Self {
        GenericVec {
            values: vec![],
            _phantom: PhantomData,
        }
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
        I::try_from_usize(len.saturating_sub(1))?;

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
        I::try_from_usize(self.len())
    }

    /// Returns a reference to the element at `index`, or an error if the index
    /// is out of range.
    pub fn get(&self, index: I) -> Result<&E, IndexOutOfRange> {
        self.values.get(index.to_usize()).ok_or(IndexOutOfRange {
            type_name: I::TYPE_NAME,
        })
    }
    /// Returns a mutable reference to the element at `index`, or an error if
    /// the index is out of range.
    pub fn get_mut(&mut self, index: I) -> Result<&mut E, IndexOutOfRange> {
        self.values
            .get_mut(index.to_usize())
            .ok_or(IndexOutOfRange {
                type_name: I::TYPE_NAME,
            })
    }

    /// Swaps two elements, or returns an error if the index is out of range.
    pub fn swap(&mut self, i: I, j: I) -> Result<(), IndexOutOfRange> {
        let i = i.to_usize();
        let j = j.to_usize();
        if i < self.len() && j < self.len() {
            self.values.swap(i, j);
            Ok(())
        } else {
            Err(IndexOutOfRange {
                type_name: I::TYPE_NAME,
            })
        }
    }

    /// Returns an iterator over the indices in the collection.
    pub fn iter_keys(&self) -> IndexIter<I> {
        IndexIter {
            range: 0..self.len(),
            _phantom: PhantomData,
        }
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

    /// Applies a function to every value in the collection and returns a new
    /// collection.
    pub fn map<U>(self, mut f: impl FnMut(I, E) -> U) -> GenericVec<I, U> {
        self.into_iter().map(|(i, e)| f(i, e)).collect()
    }
    /// Applies a function to every value in the collection and returns a new
    /// collection.
    pub fn map_ref<'a, U>(&'a self, mut f: impl FnMut(I, &'a E) -> U) -> GenericVec<I, U> {
        self.iter().map(|(i, e)| f(i, e)).collect()
    }
    /// Applies a function to every value in the collection and returns a new
    /// collection, or the first error returned by the function.
    pub fn try_map<U, S>(
        self,
        mut f: impl FnMut(I, E) -> Result<U, S>,
    ) -> Result<GenericVec<I, U>, S> {
        Ok(self
            .into_iter()
            .map(|(i, e)| f(i, e))
            .collect::<Result<Vec<_>, _>>()?
            .into())
    }
}
impl<I: IndexNewtype, E> std::iter::FromIterator<E> for GenericVec<I, E> {
    fn from_iter<T: IntoIterator<Item = E>>(iter: T) -> Self {
        let values = iter.into_iter().take(I::MAX_INDEX + 1).collect_vec();
        GenericVec {
            values,
            _phantom: PhantomData,
        }
    }
}
impl<I: IndexNewtype, E> From<Vec<E>> for GenericVec<I, E> {
    fn from(mut values: Vec<E>) -> Self {
        values.truncate(I::MAX_INDEX + 1);
        GenericVec {
            values,
            _phantom: PhantomData,
        }
    }
}

impl<I: IndexNewtype, E> IntoIterator for GenericVec<I, E> {
    type Item = (I, E);

    type IntoIter = IntoIter<I, E>;

    fn into_iter(self) -> Self::IntoIter {
        IntoIter {
            indices: self.iter_keys(),
            values: self.values.into_iter(),
        }
    }
}
/// Owning iterator over key-value pairs in a `GenericVec`.
pub struct IntoIter<I, E> {
    indices: IndexIter<I>,
    values: std::vec::IntoIter<E>,
}
impl<I: IndexNewtype, E> Iterator for IntoIter<I, E> {
    type Item = (I, E);

    fn next(&mut self) -> Option<Self::Item> {
        Some((self.indices.next()?, self.values.next()?))
    }
}

impl<'a, I: IndexNewtype, E> IntoIterator for &'a GenericVec<I, E> {
    type Item = (I, &'a E);

    type IntoIter = Iter<'a, I, E>;

    fn into_iter(self) -> Self::IntoIter {
        Iter {
            indices: self.iter_keys(),
            values: self.values.iter(),
        }
    }
}
/// Borrowing iterator over key-value pairs in a `GenericVec`.
pub struct Iter<'a, I, E> {
    indices: IndexIter<I>,
    values: std::slice::Iter<'a, E>,
}
impl<'a, I: IndexNewtype, E> Iterator for Iter<'a, I, E> {
    type Item = (I, &'a E);

    fn next(&mut self) -> Option<Self::Item> {
        Some((self.indices.next()?, self.values.next()?))
    }
}

impl<'a, I: IndexNewtype, E> IntoIterator for &'a mut GenericVec<I, E> {
    type Item = (I, &'a mut E);

    type IntoIter = IterMut<'a, I, E>;

    fn into_iter(self) -> Self::IntoIter {
        IterMut {
            indices: self.iter_keys(),
            values: self.values.iter_mut(),
        }
    }
}
/// Mutably borrowing iterator over key-value pairs in a `GenericVec`.
pub struct IterMut<'a, I, E> {
    indices: IndexIter<I>,
    values: std::slice::IterMut<'a, E>,
}
impl<'a, I: IndexNewtype, E> Iterator for IterMut<'a, I, E> {
    type Item = (I, &'a mut E);

    fn next(&mut self) -> Option<Self::Item> {
        Some((self.indices.next()?, self.values.next()?))
    }
}
