//! Approximate hash map for isometries, using a `BTreeMap` to record arbitrary
//! hash values for floats.

use ahash::HashMap;
use float_ord::FloatOrd;
use smallvec::SmallVec;
pub use std::collections::hash_map::{Entry, OccupiedEntry, VacantEntry};
use std::collections::BTreeMap;

use crate::math::cga::*;

/// Approximate hash map for isometries, using a `BTreeMap` to record arbitrary
/// hash values for floats.
#[derive(Debug, Clone)]
pub struct IsometryHashMap<V> {
    coordinate_hashes: BTreeMap<FloatOrd<f32>, FloatHash>,
    inner: HashMap<IsometryHash, V>,
}
impl<V> Default for IsometryHashMap<V> {
    fn default() -> Self {
        Self {
            coordinate_hashes: Default::default(),
            inner: Default::default(),
        }
    }
}
impl<V> IsometryHashMap<V> {
    /// Constructs an empty map.
    pub fn new() -> Self {
        IsometryHashMap::default()
    }

    /// Inserts an entry into the map and returns the old value if any.
    ///
    /// `key` is assumed to be already canonicalized.
    pub fn insert_canonicalized(&mut self, key: &Isometry, value: V) -> Option<V> {
        let hash_key = self.hash_isometry(key);
        self.inner.insert(hash_key, value)
    }
    /// Retrieves an entry from the map and returns the old value if any.
    ///
    /// `key` is assumed to be already canonicalized.
    pub fn entry_canonicalized(&mut self, key: &Isometry) -> Entry<'_, IsometryHash, V> {
        let hash_key = self.hash_isometry(key);
        self.inner.entry(hash_key)
    }

    fn hash_isometry(&mut self, m: &Isometry) -> IsometryHash {
        IsometryHash(
            m.mv()
                .terms()
                .iter()
                .map(|term| (term.axes, self.hash_float(term.coef)))
                .collect(),
        )
    }
    fn hash_float(&mut self, x: f32) -> FloatHash {
        // Search for an existing coordinate that is approximately equal to `x`.
        // If we can't find one, assign a new hash value to `x`.
        self.coordinate_hashes
            .range(FloatOrd(x - crate::math::EPSILON)..)
            .next()
            .filter(|(float, _)| float.0 <= x + crate::math::EPSILON)
            .map(|(_, &hash)| hash)
            .unwrap_or_else(|| {
                let new_hash = FloatHash(
                    self.coordinate_hashes
                        .len()
                        .try_into()
                        .expect("too many unique floats"),
                );
                self.coordinate_hashes.insert(FloatOrd(x), new_hash);
                new_hash
            })
    }
}

/// Value dervied from an isometry that can be hashed.
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
pub struct IsometryHash(SmallVec<[(Axes, FloatHash); 6]>);

/// Arbitrary hash value for a float.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
struct FloatHash(u16);
