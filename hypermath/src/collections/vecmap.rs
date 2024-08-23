use itertools::Itertools;
use num_traits::PrimInt;

/// Map implemented using an unsorted vector with linear search.
#[cfg_attr(feature = "serde", serde::Serialize, serde::Deserialize)]
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
pub struct VecMap<K, V> {
    keys: Vec<K>,
    values: Vec<V>,
}

impl<K: Eq, V> VecMap<K, V> {
    /// Constructs a new map.
    pub fn new() -> Self {
        VecMap {
            keys: vec![],
            values: vec![],
        }
    }

    /// Returns whether the map is empty.
    pub fn is_empty(&self) -> bool {
        self.keys.is_empty()
    }
    /// Returns the number of entries in the map.
    pub fn len(&self) -> usize {
        self.keys.len()
    }

    /// Inserts a new entry into the map, or replaces the existing value if the
    /// map already contains the key. The old value is returned, or `None` if
    /// the key did not exist.
    pub fn insert(&mut self, k: K, v: V) -> Option<V> {
        match self.index_of(&k) {
            Some(i) => Some(std::mem::replace(&mut self.values[i], v)),
            None => {
                self.keys.push(k);
                self.values.push(v);
                None
            }
        }
    }

    /// Returns the value associated with a given key, or `None` if the key is
    /// not in the map.
    pub fn get(&self, k: &K) -> Option<&V> {
        let i = self.index_of(k)?;
        Some(&self.values[i])
    }

    /// Returns the entry assocaited with a given key, inserting it if it is not
    /// already in the map.
    pub fn get_or_insert(&mut self, k: K, v: V) -> (&K, &mut V) {
        let i = match self.index_of(&k) {
            Some(i) => i,
            None => {
                self.keys.push(k);
                self.values.push(v);
                self.len() - 1
            }
        };
        (&self.keys[i], &mut self.values[i])
    }

    fn index_of(&self, search_key: &K) -> Option<usize> {
        Some(self.keys.iter().find_position(|&k| k == search_key)?.0)
    }

    /// Returns an iterator over all entries in the map.
    pub fn iter(&self) -> <&Self as IntoIterator>::IntoIter {
        self.into_iter()
    }
}

impl<K: Ord, V: PrimInt> VecMap<K, V> {
    /// Increments the value associated with a given key.
    pub fn increment(&mut self, k: K) {
        let v = self.get_or_insert(k, V::zero()).1;
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
    type Item = (&'a K, &'a V);

    type IntoIter = std::iter::Zip<std::slice::Iter<'a, K>, std::slice::Iter<'a, V>>;

    fn into_iter(self) -> Self::IntoIter {
        std::iter::zip(&self.keys, &self.values)
    }
}
