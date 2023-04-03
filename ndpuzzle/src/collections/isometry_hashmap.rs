use smallvec::SmallVec;

use super::approx_hashmap::*;
use crate::math::cga::{AsMultivector, Axes, Isometry};

/// Approximate hash map for isometries.
pub type IsometryHashMap<V> = ApproxHashMap<Isometry, IsometryHash, V>;

/// Value dervied from an isometry that can be hashed.
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
pub struct IsometryHash(SmallVec<[(Axes, FloatHash); 6]>);

impl<V> IsometryHashMap<V> {
    /// Constructs an empty map.
    pub fn new() -> Self {
        ApproxHashMap::default()
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
}
