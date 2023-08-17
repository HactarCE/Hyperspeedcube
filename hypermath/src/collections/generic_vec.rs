//! Vector type indexed by a newtype.

use std::fmt;
use std::marker::PhantomData;
use std::ops::{Index, IndexMut, Range};

use itertools::Itertools;

/// Error value returned by some operations related to [`GenericVec`]s when the
/// maximum value of an indexing type is exceeded.
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, Hash)]
pub struct IndexOutOfRange {
    /// Name of the indexing type.
    pub type_name: &'static str,
    /// Maximum allowed value for the indexing type.
    pub max_value: u64,
}
impl fmt::Display for IndexOutOfRange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "exceeded maximum {} count of {}",
            self.type_name, self.max_value,
        )
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
            #[derive(Debug, Default, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
            #[repr(transparent)]
            $struct_vis struct $struct_name($inner_vis $inner_type);

            impl ::std::fmt::Display for $struct_name {
                fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                    write!(f, "#{}", self.0)
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

                fn try_from_usize(index: usize) -> Result<Self, $crate::collections::generic_vec::IndexOutOfRange> {
                    match index.try_into() {
                        Ok(i) => Ok(Self(i)),
                        Err(_) => Err($crate::collections::generic_vec::IndexOutOfRange {
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
{
    /// Maximum index representable by the type.
    const MAX: Self;
    /// Maximum index representable by the type.
    const MAX_INDEX: usize;

    /// Returns the index as a `usize`.
    fn to_usize(self) -> usize {
        self.to_u64() as usize
    }

    /// Returns an index from a `usize`, or an error if it does not fit.
    fn try_from_usize(index: usize) -> Result<Self, IndexOutOfRange>;

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
    fn next(self) -> Result<Self, IndexOutOfRange> {
        Self::try_from_usize(self.to_usize().checked_add(1).unwrap_or(usize::MAX))
    }
}

/// Iterator over possible indices into a [`GenericVec<I, _>`].
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
}

/// Wrapper around a `Vec<E>` that is indexed using `I` by converting it to an
/// integer.
///
/// Elements are stored using indices.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct GenericVec<I, E> {
    values: Vec<E>,
    _phantom: PhantomData<I>,
}
impl<I: fmt::Display, E: fmt::Display> fmt::Display for GenericVec<I, E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[ {} ]", self.values.iter().join(", "))
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
    pub fn new() -> Self {
        GenericVec::default()
    }

    /// Adds an element to the end of the vector and returns its index.
    pub fn push(&mut self, value: E) -> Result<I, IndexOutOfRange> {
        let idx = self.next_idx()?;
        self.values.push(value);
        Ok(idx)
    }

    /// Extends the vector until it contains `index`.
    pub fn extend_to_contain(&mut self, index: I) -> Result<(), IndexOutOfRange>
    where
        E: Default,
    {
        while index.to_u64() >= self.len() as u64 {
            self.push(E::default())?;
        }
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
    pub fn next_idx(&self) -> Result<I, IndexOutOfRange> {
        I::try_from_usize(self.len())
    }

    /// Returns an iterator over the indices in the collection.
    pub fn iter_keys(&self) -> IndexIter<I> {
        IndexIter {
            range: 0..self.len(),
            _phantom: PhantomData,
        }
    }
    /// Returns an iterator over the values in the collection.
    pub fn iter_values(&self) -> impl Iterator<Item = &E> {
        self.values.iter()
    }
    /// Returns an iterator over the index-value pairs in the collection.
    pub fn iter(&self) -> impl Iterator<Item = (I, &E)> {
        self.iter_keys().zip(&self.values)
    }

    /// Applies a function to every value in the collection and returns a new
    /// collection.
    pub fn map<U>(&self, f: impl FnMut((I, &E)) -> U) -> GenericVec<I, U> {
        self.iter().map(f).collect()
    }
}
impl<I: IndexNewtype, E> std::iter::FromIterator<E> for GenericVec<I, E> {
    fn from_iter<T: IntoIterator<Item = E>>(iter: T) -> Self {
        let values = iter.into_iter().collect_vec();
        assert!(values.len() <= I::MAX_INDEX);
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
