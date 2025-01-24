use std::collections::HashMap;

use hypermath::collections::{GenericVec, IndexNewtype, IndexOutOfRange, IndexOverflow};
use itertools::Itertools;

use super::NameSet;

/// Mutable ordering of elements. By default, elements remain in insertion
/// order.
#[derive(Debug, Default, Clone)]
pub struct CustomOrdering<I> {
    index_by_id: GenericVec<I, usize>,
    id_by_index: Vec<I>,
}
impl<I: IndexNewtype> CustomOrdering<I> {
    /// Constructs a new ordering with no elements.
    pub fn new() -> Self {
        Self::default()
    }

    /// Constructs a new ordering with `len` elements.
    pub fn with_len(len: usize) -> Result<Self, IndexOverflow> {
        let mut ret = Self::new();
        for id in I::iter(len) {
            ret.add(id)?;
        }
        Ok(ret)
    }

    /// Adds an element to the ordering.
    ///
    /// If this ordering is part of a larger structure, this will be called
    /// automatically by that larger structure.
    pub fn add(&mut self, id: I) -> Result<(), IndexOverflow> {
        let index = self.index_by_id.len();
        self.index_by_id.push(index)?;
        self.id_by_index.push(id);
        Ok(())
    }

    /// Returns the index of the element with the given ID, or an error if the
    /// index is out of range.
    pub fn get_index(&self, id: I) -> Result<usize, IndexOutOfRange> {
        self.index_by_id.get(id).copied()
    }
    /// Returns the ID of the element at the given index, or an error if the ID
    /// is out of range.
    pub fn id_from_index(&self, index: usize) -> Result<I, IndexOutOfRange> {
        self.id_by_index.get(index).copied().ok_or(IndexOutOfRange {
            type_name: I::TYPE_NAME,
        })
    }

    /// Moves an element to the index of another element, shifting all elements
    /// in between. If either index is out of bounds, this method does nothing.
    pub fn shift_to(&mut self, from: I, to: I) {
        let Ok(&i) = self.index_by_id.get(from) else {
            return;
        };
        let Ok(&j) = self.index_by_id.get(to) else {
            return;
        };
        if i < j {
            for k in i..j {
                self.swap_indices_unchecked(k, k + 1);
            }
        }
        if j < i {
            for k in (j..i).rev() {
                self.swap_indices_unchecked(k, k + 1);
            }
        }
    }

    /// Swaps two elements in the canonical order. If either index is out of
    /// bounds, this method does nothing.
    pub fn swap(&mut self, a: I, b: I) {
        let Ok(&i) = self.index_by_id.get(a) else {
            return;
        };
        let Ok(&j) = self.index_by_id.get(b) else {
            return;
        };
        self.index_by_id
            .swap(a, b)
            .expect("bad index in CustomOrdering::swap()");
        self.id_by_index.swap(i, j);
    }

    fn swap_indices_unchecked(&mut self, i: usize, j: usize) {
        let a = self.id_by_index[i];
        let b = self.id_by_index[j];
        self.index_by_id
            .swap(a, b)
            .expect("bad index in CustomOrdering::swap_indices_unchecked()");
        self.id_by_index.swap(i, j);
    }

    /// Swaps the element `i` with the element at index `j` in the canonical
    /// order, or returns an error if the index `j` is out of bounds. Call this
    /// multiple times with increasing `j` to force a particular ordering
    /// globally.
    pub fn swap_to_index(&mut self, i: I, j: usize) -> Result<(), IndexOutOfRange> {
        self.swap(i, self.id_from_index(j)?);
        Ok(())
    }

    /// Sorts the list lexicographically by name (case-sensitive). If multiple
    /// elements have no name, they will be placed at the beginning of the list
    /// in the order they were defined.
    pub fn sort_by_name(&mut self, id_to_name: &HashMap<I, NameSet>) {
        let new_order = self
            .index_by_id
            .iter_keys()
            .sorted_by_key(|id| id_to_name.get(id)?.canonical_name());
        for (index, id) in new_order.enumerate() {
            // Ignore errors; it doesn't matter.
            let _ = self.swap_to_index(id, index);
        }
    }

    /// Returns the list of IDs in order.
    pub fn ids_in_order(&self) -> &[I] {
        &self.id_by_index
    }

    /// Reorders all elements to match `new_order`.
    ///
    /// This method will always result in a reasonable state, but may give an
    /// unstable sort if `new_order` does not include all elements.
    ///
    /// Returns an error if any index is out of range.
    pub fn reorder_all(
        &mut self,
        new_order: impl IntoIterator<Item = I>,
    ) -> Result<(), IndexOutOfRange> {
        for (index, id) in new_order.into_iter().enumerate() {
            self.swap_to_index(id, index)?;
        }
        Ok(())
    }
}
impl<'a, I: IndexNewtype> IntoIterator for &'a CustomOrdering<I> {
    type Item = I;

    type IntoIter = std::iter::Copied<std::slice::Iter<'a, I>>;

    fn into_iter(self) -> Self::IntoIter {
        self.ids_in_order().iter().copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    hypermath::idx_struct! {
        struct Index(pub u8);
    }

    #[test]
    fn test_custom_ordering() {
        let mut ordering = CustomOrdering::new();
        ordering.add(Index(0)).unwrap();
        ordering.add(Index(1)).unwrap();
        ordering.add(Index(2)).unwrap();
        ordering.add(Index(3)).unwrap();
        ordering.add(Index(4)).unwrap();

        // Test `.swap()`.
        ordering.swap(Index(1), Index(2));
        ordering.swap(Index(1), Index(3));
        let expected = [0, 2, 3, 1, 4].map(Index);
        assert_eq!(ordering.ids_in_order(), &expected);

        // Test `.get_index()` and `.id_from_index()`.
        for (i, v) in expected.into_iter().enumerate() {
            assert_eq!(ordering.get_index(v).unwrap(), i);
            assert_eq!(ordering.id_from_index(i).unwrap(), v);
        }
        assert!(ordering.get_index(Index(5)).is_err());
        assert!(ordering.id_from_index(5).is_err());

        // Test `.shift_to()` going up.
        ordering.shift_to(Index(2), Index(1));
        assert_eq!(ordering.ids_in_order(), &[0, 3, 1, 2, 4].map(Index));
        // Test `.shift_to()` going down.
        ordering.shift_to(Index(2), Index(3));
        assert_eq!(ordering.ids_in_order(), &[0, 2, 3, 1, 4].map(Index));

        // Test `.swap_to_index()`. It should be idempotent.
        ordering.swap_to_index(Index(4), 1).unwrap();
        assert_eq!(ordering.ids_in_order(), &[0, 4, 3, 1, 2].map(Index));
        ordering.swap_to_index(Index(4), 1).unwrap();
        assert_eq!(ordering.ids_in_order(), &[0, 4, 3, 1, 2].map(Index));

        // Test `.reorder_all()`.
        ordering.reorder_all([4, 0, 1, 3, 2].map(Index)).unwrap();
        assert_eq!(ordering.ids_in_order(), &[4, 0, 1, 3, 2].map(Index));
        ordering.reorder_all([4, 2, 0, 1].map(Index)).unwrap();
        assert_eq!(ordering.ids_in_order(), &[4, 2, 0, 1, 3].map(Index));

        let id_to_name = HashMap::from_iter(
            ["o kama", "sona", "e", "toki", "pona"]
                .into_iter()
                .enumerate()
                .map(|(i, name)| (Index(i as u8), NameSet::from(name))),
        );
        ordering.sort_by_name(&id_to_name);
        assert_eq!(ordering.ids_in_order(), &[2, 0, 4, 1, 3].map(Index))
    }
}
