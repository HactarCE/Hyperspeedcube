use itertools::Itertools;
use num_traits::PrimInt;

/// Map implemented using an unsorted vector with linear search.
#[cfg_attr(feature = "serde", serde::Serialize, serde::Deserialize)]
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
pub struct VecMap<K, V> {
    keys: Vec<K>,
    values: Vec<V>,
}
impl<K: Ord, V> VecMap<K, V> {
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
    /// map already contains the key.
    pub fn insert(&mut self, k: K, v: V) {
        match self.index_of(&k) {
            Some(i) => self.values[i] = v,
            None => {
                self.keys.push(k);
                self.values.push(v);
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
}
impl<K: Ord, V: PrimInt> VecMap<K, V> {
    /// Increments the value associated with a given key.
    pub fn increment(&mut self, k: K) {
        let v = self.get_or_insert(k, V::zero()).1;
        *v = *v + V::one();
    }
}
