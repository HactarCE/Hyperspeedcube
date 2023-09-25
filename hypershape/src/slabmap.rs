use std::{collections::hash_map, marker::PhantomData, ops::Index};

use hypermath::{
    collections::{approx_hashmap::ApproxHashMapKey, generic_vec::IndexOutOfRange, ApproxHashMap},
    IndexNewtype,
};
use slab::Slab;

trait HashMappable {}

/// Data structure that assigns an ID to each unique element inserted into it
/// and allows efficient lookups in both directions.
#[derive(Debug, Default, Clone)]
pub struct SlabMap<I: IndexNewtype, T: ApproxHashMapKey> {
    /// Flat array of values.
    slab: Slab<T>,
    /// Hashmap to convert from value to ID.
    hashmap: ApproxHashMap<T, I>,

    /// Useless [`PhantomData`] to fix `#[derive(Debug)]` bounds.
    _phantom: PhantomData<T::Hash>,
}
impl<I: IndexNewtype, T: ApproxHashMapKey> Index<I> for SlabMap<I, T> {
    type Output = T;

    fn index(&self, index: I) -> &Self::Output {
        &self.slab[index.to_usize()]
    }
}
impl<I: IndexNewtype, T: Clone + ApproxHashMapKey> SlabMap<I, T> {
    /// Constructs an empty `SlabMap`.
    pub fn new() -> Self {
        SlabMap {
            slab: Slab::new(),
            hashmap: ApproxHashMap::new(),
            _phantom: PhantomData,
        }
    }

    /// Returns the ID for a value, adding it to the map and assigning it an ID
    /// if it doesn't already have one.
    pub fn get_or_insert(&mut self, value: T) -> Result<SlabMapEntry<I>, IndexOutOfRange> {
        match self.hashmap.entry(&value) {
            hash_map::Entry::Occupied(e) => Ok(SlabMapEntry::Old(*e.get())),
            hash_map::Entry::Vacant(e) => {
                let index = I::try_from_usize(self.slab.insert(value.clone()))?;
                Ok(SlabMapEntry::New(*e.insert(index)))
            }
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum SlabMapEntry<I> {
    New(I),
    Old(I),
}
impl<I: IndexNewtype> SlabMapEntry<I> {
    /// Returns the automatically-assigned key for the entry.
    pub fn key(self) -> I {
        match self {
            SlabMapEntry::New(index) => index,
            SlabMapEntry::Old(index) => index,
        }
    }
    /// Returns `true` if the value is new and `false` if the value was already
    /// present.
    pub fn is_new(self) -> bool {
        match self {
            SlabMapEntry::New(_) => true,
            SlabMapEntry::Old(_) => false,
        }
    }
}
