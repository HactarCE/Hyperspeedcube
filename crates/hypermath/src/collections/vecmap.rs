use std::borrow::Borrow;

use itertools::Itertools;
use num_traits::PrimInt;

/// Map implemented using an unsorted vector with linear search.
#[cfg_attr(feature = "serde", serde::Serialize, serde::Deserialize)]
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
pub struct VecMap<K, V> {
    entries: Vec<VecMapEntry<K, V>>,
}

#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, Hash)]
pub struct VecMapEntry<K, V> {
    key: K,
    pub value: V,
}
impl<K, V> VecMapEntry<K, V> {
    pub fn key(&self) -> &K {
        &self.key
    }
}

impl<K: Eq, V> VecMap<K, V> {
    /// Constructs a new map.
    pub fn new() -> Self {
        VecMap { entries: vec![] }
    }

    /// Returns whether the map is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
    /// Returns the number of entries in the map.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Inserts a new entry into the map, or replaces the existing value if the
    /// map already contains the key. The old value is returned, or `None` if
    /// the key did not exist.
    pub fn insert(&mut self, k: K, v: V) -> Option<V> {
        match self.index_of(&k) {
            Some(i) => Some(std::mem::replace(&mut self.entries[i].value, v)),
            None => {
                self.entries.push(VecMapEntry { key: k, value: v });
                None
            }
        }
    }

    /// Returns the value associated with a given key, or `None` if the key is
    /// not in the map.
    pub fn get<Q: Eq + ?Sized>(&self, k: &Q) -> Option<&V>
    where
        K: Borrow<Q>,
    {
        let i = self.index_of(k)?;
        Some(&self.entries[i].value)
    }

    /// Returns the entry associated with a given key, inserting it if it is not
    /// already in the map.
    pub fn get_or_insert(&mut self, k: K, v: V) -> &mut VecMapEntry<K, V> {
        let i = match self.index_of(&k) {
            Some(i) => i,
            None => {
                self.entries.push(VecMapEntry { key: k, value: v });
                self.len() - 1
            }
        };
        &mut self.entries[i]
    }

    /// Removes the entry associated with a given key, and returns the value if
    /// it was present.
    pub fn remove<Q: Eq + ?Sized>(&mut self, k: &Q) -> Option<V>
    where
        K: Borrow<Q>,
    {
        let i = self.index_of(k)?;
        Some(self.entries.swap_remove(i).value)
    }

    fn index_of<Q: Eq + ?Sized>(&self, search_key: &Q) -> Option<usize>
    where
        K: Borrow<Q>,
    {
        Some(self.keys().find_position(|&k| k.borrow() == search_key)?.0)
    }

    // /// Returns an iterator over all entries in the map.
    // pub fn iter(&self) -> <&Self as IntoIterator>::IntoIter {
    //     self.into_iter()
    // }

    /// Returns an iterator over the keys.
    pub fn keys(&self) -> impl Iterator<Item = &K> {
        self.entries.iter().map(|e| e.key())
    }
}

impl<K: Ord, V: PrimInt> VecMap<K, V> {
    /// Increments the value associated with a given key.
    pub fn increment(&mut self, k: K) {
        let v = &mut self.get_or_insert(k, V::zero()).value;
        *v = *v + V::one();
    }
}

impl<K: Eq, V> FromIterator<(K, V)> for VecMap<K, V> {
    fn from_iter<T: IntoIterator<Item = (K, V)>>(iter: T) -> Self {
        let mut ret = VecMap::new();
        for (k, v) in iter {
            ret.insert(k, v);
        }
        ret
    }
}

impl<'a, K, V> IntoIterator for &'a VecMap<K, V> {
    type Item = &'a VecMapEntry<K, V>;

    type IntoIter = std::slice::Iter<'a, VecMapEntry<K, V>>;

    fn into_iter(self) -> Self::IntoIter {
        self.entries.iter()
    }
}
