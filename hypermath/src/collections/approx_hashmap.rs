//! Approximate hash map for floating-point values such as vectors, using a
//! `BTreeMap` to record arbitrary hash values for floats.

pub use std::collections::hash_map::{Entry, OccupiedEntry, VacantEntry};
use std::collections::{BTreeMap, HashMap};
use std::hash::Hash;
use std::marker::PhantomData;

use float_ord::FloatOrd;
use smallvec::SmallVec;

use crate::*;

/// Arbitrary hash value for a float.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct FloatHash(u16);

/// Approximate hash map for objects with floating-point values, using a
/// `BTreeMap` to record arbitrary hash values for floats.
#[derive(Debug, Clone)]
pub struct ApproxHashMap<K: ApproxHashMapKey, V> {
    pub(crate) inner: HashMap<K::Hash, V>,
    float_hashes: BTreeMap<FloatOrd<Float>, FloatHash>,
    _phantom: PhantomData<K>,
}
impl<K: ApproxHashMapKey, V> Default for ApproxHashMap<K, V> {
    fn default() -> Self {
        Self {
            inner: Default::default(),
            float_hashes: Default::default(),
            _phantom: PhantomData,
        }
    }
}
impl<K: ApproxHashMapKey, V> ApproxHashMap<K, V> {
    /// Constructs an empty map.
    pub fn new() -> Self {
        ApproxHashMap::default()
    }

    /// Inserts an entry into the map and returns the old value, if any.
    ///
    /// `key` is assumed to be already canonicalized, if necessary. For example,
    /// most multivectors should be normalized before being stored in an
    /// `ApproxHashMap`.
    pub fn insert(&mut self, key: &K, value: V) -> Option<V> {
        let hash_key = key.approx_hash(|x| self.hash_float(x));
        self.inner.insert(hash_key, value)
    }
    /// Retrieves an entry from the map and returns the old value, if any.
    ///
    /// `key` is assumed to be already canonicalized, if necessary. For example,
    /// most multivectors should be normalized before being stored in an
    /// `ApproxHashMap`.
    pub fn entry(&mut self, key: &K) -> Entry<'_, K::Hash, V> {
        let hash_key = key.approx_hash(|x| self.hash_float(x));
        self.inner.entry(hash_key)
    }

    /// Search for an existing hash value for a float that is approximately
    /// equal to `x`, and returns it if found. If none is found, assign a new
    /// hash value to `x` and returns that.
    fn hash_float(&mut self, x: Float) -> FloatHash {
        self.float_hashes
            .range(FloatOrd(x - EPSILON)..=FloatOrd(x + EPSILON))
            .next()
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

/// Type that can be used as a key in an [`ApproxHashMap`].
pub trait ApproxHashMapKey {
    /// Hashable representation of the type, using [`FloatHash`] instead of any
    /// floating-point values.
    type Hash: Eq + Hash;

    /// Returns a hashable representation of a value, using [`FloatHash`]
    /// instead of any floating-point values.
    fn approx_hash(&self, float_hash_fn: impl FnMut(Float) -> FloatHash) -> Self::Hash;
}

/// Value dervied from a floating-point vector that can be hashed. Don't use
/// this directly; use via [`ApproxHashMap`].
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
pub struct VectorHash(SmallVec<[(u8, FloatHash); 6]>);

impl ApproxHashMapKey for Vector {
    type Hash = VectorHash;

    fn approx_hash(&self, mut float_hash_fn: impl FnMut(Float) -> FloatHash) -> Self::Hash {
        let hash_elem = |(i, x)| (i, float_hash_fn(x));
        VectorHash(self.iter_nonzero().map(hash_elem).collect())
    }
}

/// Value dervied from a multivector that can be hashed. Don't use this
/// directly; use via [`ApproxHashMap`].
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
pub struct MultivectorHash(SmallVec<[(Axes, FloatHash); 6]>);

impl<T: Clone + Eq + Hash> ApproxHashMapKey for T {
    type Hash = T;

    fn approx_hash(&self, _float_hash_fn: impl FnMut(Float) -> FloatHash) -> Self::Hash {
        self.clone()
    }
}

impl ApproxHashMapKey for Multivector {
    type Hash = MultivectorHash;

    fn approx_hash(&self, mut float_hash_fn: impl FnMut(Float) -> FloatHash) -> Self::Hash {
        let hash_term = |Term { axes, coef }| (axes, float_hash_fn(coef));
        MultivectorHash(self.nonzero_terms().map(hash_term).collect())
    }
}
impl ApproxHashMapKey for Blade {
    type Hash = MultivectorHash;

    fn approx_hash(&self, float_hash_fn: impl FnMut(Float) -> FloatHash) -> Self::Hash {
        self.mv().approx_hash(float_hash_fn)
    }
}
impl ApproxHashMapKey for Isometry {
    type Hash = MultivectorHash;

    fn approx_hash(&self, float_hash_fn: impl FnMut(Float) -> FloatHash) -> Self::Hash {
        self.mv().approx_hash(float_hash_fn)
    }
}
