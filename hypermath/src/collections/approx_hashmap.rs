//! Approximate hash map for floating-point values such as vectors, using a
//! `BTreeMap` to record arbitrary hash values for floats.

pub use std::collections::hash_map::{Entry, Iter, OccupiedEntry, VacantEntry};
use std::collections::{BTreeMap, HashMap};
use std::fmt;
use std::hash::Hash;
use std::marker::PhantomData;

use float_ord::FloatOrd;
use smallvec::SmallVec;

use crate::*;

/// Arbitrary hash value for a float.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct FloatHash(u32);

/// Approximate hash map for objects with floating-point values, using a
/// `BTreeMap` to record arbitrary hash values for floats.
///
/// Multivectors representing flectors (rotors & reflectors) must be
/// canonicalized before being inserted into the map. Vectors are automatically
/// canonicalized by ignoring components that equal zero.
#[derive(Clone)]
pub struct ApproxHashMap<K: ApproxHashMapKey, V> {
    keys: HashMap<K::Hash, K>,
    values: HashMap<K::Hash, V>,
    float_hashes: BTreeMap<FloatOrd<Float>, FloatHash>,
    _phantom: PhantomData<K>,
}
impl<K: ApproxHashMapKey, V> Default for ApproxHashMap<K, V> {
    fn default() -> Self {
        Self {
            keys: HashMap::new(),
            values: HashMap::new(),
            float_hashes: BTreeMap::new(),
            _phantom: PhantomData,
        }
    }
}
impl<K: ApproxHashMapKey + fmt::Debug, V: fmt::Debug> fmt::Debug for ApproxHashMap<K, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let entries = self.keys.iter().map(|(hash, k)| (k, self.values.get(hash)));
        f.debug_map().entries(entries).finish()
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
    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        let hash_key = key.approx_hash(|x| self.hash_float(x));
        self.keys.insert(hash_key.clone(), key);
        self.values.insert(hash_key, value)
    }
    /// Retrieves an entry from the map and returns the old value, if any.
    ///
    /// `key` is assumed to be already canonicalized, if necessary. For example,
    /// most multivectors should be normalized before being stored in an
    /// `ApproxHashMap`.
    pub fn entry(&mut self, key: K) -> Entry<'_, K::Hash, V> {
        let hash_key = key.approx_hash(|x| self.hash_float(x));
        self.keys.insert(hash_key.clone(), key);
        self.values.entry(hash_key)
    }
    /// Returns the value in the map associated to the given key (or something
    /// approximately equal).
    pub fn get(&self, key: &K) -> Option<&V> {
        let hash_key = key.try_approx_hash(|x| self.try_hash_float(x))?;
        self.values.get(&hash_key)
    }

    /// Searches for an existing hash value for a float that is approximately
    /// equal to `x`, and returns it if found.
    fn try_hash_float(&self, x: Float) -> Option<FloatHash> {
        self.float_hashes
            .range(FloatOrd(x - EPSILON)..=FloatOrd(x + EPSILON))
            .next()
            .map(|(_, &hash)| hash)
    }
    /// Searches for an existing hash value for a float that is approximately
    /// equal to `x`, and returns it if found. If none is found, assign a new
    /// hash value to `x` and returns that.
    fn hash_float(&mut self, x: Float) -> FloatHash {
        self.try_hash_float(x).unwrap_or_else(|| {
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

    /// Returns an iterator over all keys and values in the map.
    pub fn iter(&self) -> impl Iterator<Item = (&K, &V)> {
        self.values.iter().map(|(k, v)| (&self.keys[k], v))
    }
}

/// Type that can be used as a key in an [`ApproxHashMap`].
pub trait ApproxHashMapKey {
    /// Hashable representation of the type, using [`FloatHash`] instead of any
    /// floating-point values.
    type Hash: fmt::Debug + Clone + Eq + Hash;

    /// Returns a hashable representation of a value, using [`FloatHash`]
    /// instead of any floating-point values.
    fn approx_hash(&self, float_hash_fn: impl FnMut(Float) -> FloatHash) -> Self::Hash;

    /// Returns a hashable representation of a value, or `None` if
    /// `float_hash_fn` ever returns `None`.
    fn try_approx_hash(
        &self,
        mut float_hash_fn: impl FnMut(Float) -> Option<FloatHash>,
    ) -> Option<Self::Hash> {
        let mut success = true;
        Some(self.approx_hash(|x| match float_hash_fn(x) {
            Some(h) => h,
            None => {
                success = false;
                FloatHash(0)
            }
        }))
        .filter(|_| success)
    }
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

/// Value dervied from a CGA multivector that can be hashed. Don't use this
/// directly; use via [`ApproxHashMap`].
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
pub struct MultivectorHash(SmallVec<[(u16, FloatHash); 6]>);

impl<T: ApproxHashMapKey> ApproxHashMapKey for Option<T> {
    type Hash = Option<T::Hash>;

    fn approx_hash(&self, float_hash_fn: impl FnMut(Float) -> FloatHash) -> Self::Hash {
        self.as_ref().map(|x| x.approx_hash(float_hash_fn))
    }
}
impl<T: ApproxHashMapKey> ApproxHashMapKey for Vec<T> {
    type Hash = Vec<T::Hash>;

    fn approx_hash(&self, mut float_hash_fn: impl FnMut(Float) -> FloatHash) -> Self::Hash {
        self.iter()
            .map(|x| x.approx_hash(&mut float_hash_fn))
            .collect()
    }
}

impl ApproxHashMapKey for cga::Multivector {
    type Hash = MultivectorHash;

    fn approx_hash(&self, mut float_hash_fn: impl FnMut(Float) -> FloatHash) -> Self::Hash {
        let hash_term = |cga::Term { axes, coef }| (axes.bits(), float_hash_fn(coef));
        MultivectorHash(self.nonzero_terms().map(hash_term).collect())
    }
}
impl ApproxHashMapKey for cga::Blade {
    type Hash = MultivectorHash;

    fn approx_hash(&self, float_hash_fn: impl FnMut(Float) -> FloatHash) -> Self::Hash {
        self.mv().approx_hash(float_hash_fn)
    }
}
impl ApproxHashMapKey for cga::Isometry {
    type Hash = MultivectorHash;

    fn approx_hash(&self, float_hash_fn: impl FnMut(Float) -> FloatHash) -> Self::Hash {
        self.mv().approx_hash(float_hash_fn)
    }
}

impl ApproxHashMapKey for pga::Blade {
    type Hash = MultivectorHash;

    fn approx_hash(&self, mut float_hash_fn: impl FnMut(Float) -> FloatHash) -> Self::Hash {
        let hash_term = |pga::Term { coef, axes }| (axes.bits() as u16, float_hash_fn(coef));
        MultivectorHash(self.nonzero_terms().map(hash_term).collect())
    }
}
impl ApproxHashMapKey for pga::Motor {
    type Hash = MultivectorHash;

    fn approx_hash(&self, mut float_hash_fn: impl FnMut(Float) -> FloatHash) -> Self::Hash {
        let hash_term = |pga::Term { coef, axes }| (axes.bits() as u16, float_hash_fn(coef));
        MultivectorHash(self.nonzero_terms().map(hash_term).collect())
    }
}
