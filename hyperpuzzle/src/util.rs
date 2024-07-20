use std::collections::HashMap;
use std::fmt;
use std::hash::Hash;

use itertools::Itertools;

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

/// Titlecases a string, replacing underscore `_` with space.
pub fn titlecase(s: &str) -> String {
    s.split(&[' ', '_'])
        .filter(|word| !word.is_empty())
        .map(|word| {
            if let Some((char_boundary, _)) = word.char_indices().skip(1).next() {
                let (left, right) = word.split_at(char_boundary);
                left.to_uppercase() + right
            } else {
                word.to_uppercase()
            }
        })
        .join(" ")
}

pub fn lazy_resolve<K: fmt::Debug + Clone + Eq + Hash, V: Clone>(
    key_value_dependencies: Vec<(K, (V, Option<K>))>,
    compose: fn(V, &V) -> V,
    warn_fn: impl Fn(String),
) -> HashMap<K, V> {
    // Some values are given directly.
    let mut known = Vec::<(K, V)>::new();
    // Some must be computed based on other values.
    let mut unknown = HashMap::<K, Vec<(K, V)>>::new();

    for (k, (v, other_key)) in key_value_dependencies {
        match other_key {
            Some(k2) => unknown.entry(k2).or_default().push((k, v)),
            None => known.push((k, v)),
        }
    }

    let mut known: HashMap<K, V> = known.iter().cloned().collect();

    // Resolve lazy evaluation.
    let mut queue = known.iter().map(|(k, _v)| k.clone()).collect_vec();
    while let Some(next_known) = queue.pop() {
        if let Some(unprocessed) = unknown.remove(&next_known) {
            for (k, v) in unprocessed {
                let value = compose(v, &known[&next_known]);
                known.insert(k.clone(), value);
                queue.push(k);
            }
        }
    }
    if let Some(unprocessed_key) = unknown.keys().next() {
        warn_fn(format!("unknown key {unprocessed_key:?}"));
    }

    known
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

    #[test]
    fn test_titlecase() {
        assert_eq!(titlecase("  this_was a__triumph_"), "This Was A Triumph");
    }
}