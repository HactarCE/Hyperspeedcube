use std::collections::HashMap;

/// Returns an iterator over the strings `A`, `B`, `C`, ..., `Z`, `AA`, `AB`,
/// ..., `ZY`, `ZZ`, `AAA`, `AAB`, etc.
pub(crate) fn iter_uppercase_letter_names() -> impl Iterator<Item = String> {
    (1..).flat_map(|len| {
        (0..26_usize.pow(len)).map(move |i| {
            (0..len)
                .rev()
                .map(|j| ('A' as u8 + ((i / 26_usize.pow(j)) % 26) as u8) as char)
                .collect()
        })
    })
}

/// Maps IDs from one list to another.
pub struct LazyIdMap<K, V> {
    map: HashMap<K, V>,
    keys: Vec<K>,
    next_id: V,
    next_id_fn: fn(V) -> V,
}
impl<K: Clone + std::hash::Hash + Eq, V: Copy> LazyIdMap<K, V> {
    pub fn new(first_id: V, next_id_fn: fn(V) -> V) -> Self {
        Self {
            map: HashMap::new(),
            keys: vec![],
            next_id: first_id,
            next_id_fn,
        }
    }
    pub fn get_or_insert(&mut self, id: K) -> V {
        match self.map.entry(id.clone()) {
            std::collections::hash_map::Entry::Occupied(e) => *e.get(),
            std::collections::hash_map::Entry::Vacant(e) => {
                self.keys.push(id);
                let new_id = self.next_id;
                self.next_id = (self.next_id_fn)(self.next_id);
                *e.insert(new_id)
            }
        }
    }
    pub fn keys(&self) -> &[K] {
        &self.keys
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_iter_letter_names() {
        let mut it = iter_uppercase_letter_names();
        assert_eq!(it.next().unwrap(), "A");
        assert_eq!(it.next().unwrap(), "B");
        let mut it = it.skip(23);
        assert_eq!(it.next().unwrap(), "Z");
        assert_eq!(it.next().unwrap(), "AA");
        assert_eq!(it.next().unwrap(), "AB");
        let mut it = it.skip(23);
        assert_eq!(it.next().unwrap(), "AZ");
        assert_eq!(it.next().unwrap(), "BA");
        assert_eq!(it.next().unwrap(), "BB");
        assert_eq!(it.next().unwrap(), "BC");
        let mut it = it.skip(645);
        assert_eq!(it.next().unwrap(), "ZY");
        assert_eq!(it.next().unwrap(), "ZZ");
        assert_eq!(it.next().unwrap(), "AAA");
        assert_eq!(it.next().unwrap(), "AAB");
    }
}
