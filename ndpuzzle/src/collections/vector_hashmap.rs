use smallvec::SmallVec;

use super::approx_hashmap::*;
use crate::math::{Vector, VectorRef};

/// Approximate hash map for vecotrs.
pub type VectorHashMap<V> = ApproxHashMap<Vector, VectorHash, V>;

/// Value dervied from a floating-point vector that can be hashed.
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
pub struct VectorHash(SmallVec<[FloatHash; 8]>);

impl<V> VectorHashMap<V> {
    /// Constructs an empty map.
    pub fn new() -> Self {
        ApproxHashMap::default()
    }

    /// Inserts an entry into the map and returns the old value if any.
    pub fn insert(&mut self, key: impl VectorRef, value: V) -> Option<V> {
        let hash_key = self.hash_vector(key);
        self.inner.insert(hash_key, value)
    }
    /// Retrieves an entry from the map and returns the old value if any.
    pub fn entry(&mut self, key: impl VectorRef) -> Entry<'_, VectorHash, V> {
        let hash_key = self.hash_vector(key);
        self.inner.entry(hash_key)
    }

    fn hash_vector(&mut self, v: impl VectorRef) -> VectorHash {
        VectorHash(v.iter().map(|x| self.hash_float(x)).collect())
    }
}
