//! Approximate hash map for floating-point values such as vectors, using a
//! `BTreeMap` to record arbitrary hash values for floats.

use ahash::HashMap;
use float_ord::FloatOrd;
pub use std::collections::hash_map::{Entry, OccupiedEntry, VacantEntry};
use std::{collections::BTreeMap, hash::Hash, marker::PhantomData};

/// Arbitrary hash value for a float.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub(super) struct FloatHash(u16);

/// Approximate hash map for floating-point vectors, using a `BTreeMap` to
/// record arbitrary hash values for floats.
#[derive(Debug, Clone)]
pub struct ApproxHashMap<K, H, V> {
    pub(super) inner: HashMap<H, V>,
    float_hashes: BTreeMap<FloatOrd<f32>, FloatHash>,
    _phantom: PhantomData<K>,
}
impl<K, H, V> Default for ApproxHashMap<K, H, V> {
    fn default() -> Self {
        Self {
            inner: Default::default(),
            float_hashes: Default::default(),
            _phantom: PhantomData,
        }
    }
}
impl<K, H, V> ApproxHashMap<K, H, V> {
    pub(super) fn hash_float(&mut self, x: f32) -> FloatHash {
        // Search for an existing coordinate that is approximately equal to `x`.
        // If we can't find one, assign a new hash value to `x`.
        self.float_hashes
            .range(FloatOrd(x - crate::math::EPSILON)..)
            .next()
            .filter(|(float, _)| float.0 <= x + crate::math::EPSILON)
            .map(|(_, &hash)| hash)
            .unwrap_or_else(|| {
                let new_hash = FloatHash(
                    self.float_hashes
                        .len()
                        .try_into()
                        .expect("too many unique floats"),
                );
                self.float_hashes.insert(FloatOrd(x), new_hash);
                new_hash
            })
    }
}
