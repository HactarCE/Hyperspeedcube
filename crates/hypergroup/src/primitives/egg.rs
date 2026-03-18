use std::fmt;

use hypuz_util::ti::TypedIndex;
use itertools::Itertools;

use crate::{GeneratorId, GroupElementId, GroupError, GroupResult, PerGenerator, PerGroupElement};

/// Element-generator group table.
///
/// 2D array containing a value for each possible element+generator pairing.
#[derive(Clone, PartialEq, Eq, Hash)]
pub(crate) struct EggTable<T> {
    /// Number of elements in the group.
    element_count: usize,
    /// Number of generators in the group.
    generator_count: usize,
    /// Flattened 2D array, indexed by a pair of element ID and value index.
    contents: Vec<T>,
}

impl<T: fmt::Debug> fmt::Debug for EggTable<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list()
            .entries(GroupElementId::iter(self.element_count).map(|e| {
                GeneratorId::iter(self.generator_count)
                    .map(|g| self.get(e, g))
                    .collect_vec()
            }))
            .finish()
    }
}

impl<T> EggTable<T> {
    /// Constructs a new EGG table containing only the identity.
    pub fn new(generator_count: usize) -> Self
    where
        T: Default + Clone,
    {
        EggTable {
            element_count: 1,
            generator_count,
            contents: vec![T::default(); generator_count],
        }
    }

    /// Adds a new element to the table.
    pub fn add_element(&mut self) -> GroupResult<GroupElementId>
    where
        T: Default + Clone,
    {
        let new_element = GroupElementId::try_from_index(self.element_count)?;
        self.element_count += 1;
        self.contents
            .resize(self.element_count * self.generator_count, T::default());
        Ok(new_element)
    }

    /// Returns a value from the table.
    #[inline]
    #[track_caller]
    pub fn get(&self, element: GroupElementId, generator: GeneratorId) -> &T {
        &self.contents[self.index(element, generator)]
    }

    /// Returns a mutable reference to a value in the table.
    #[track_caller]
    pub fn get_mut(&mut self, element: GroupElementId, generator: GeneratorId) -> &mut T {
        let index = self.index(element, generator);
        &mut self.contents[index]
    }

    /// Returns an integer index into `contents`.
    #[inline]
    #[track_caller]
    fn index(&self, element: GroupElementId, generator: GeneratorId) -> usize {
        assert!(
            (generator.0 as usize) < self.generator_count,
            "generator {generator} out of range (max {max})",
            max = self.generator_count,
        );
        element.0 as usize * self.generator_count + generator.0 as usize
    }

    /// Returns an iterator over keys and values.
    pub fn iter(&self) -> impl '_ + Iterator<Item = ((GroupElementId, GeneratorId), &T)> {
        let elements_iter = GroupElementId::iter(self.element_count);
        let generators_iter = GeneratorId::iter(self.generator_count);
        itertools::iproduct!(elements_iter, generators_iter).zip(&self.contents)
    }
}

impl<T> EggTable<Option<T>> {
    pub fn try_unwrap(self) -> GroupResult<EggTable<T>> {
        match self.contents.into_iter().collect::<Option<Vec<T>>>() {
            Some(contents) => Ok(EggTable {
                element_count: self.element_count,
                generator_count: self.generator_count,
                contents,
            }),
            None => Err(GroupError::IncompleteGroupStructure),
        }
    }
}

impl EggTable<GroupElementId> {
    /// Performs basic sanity checks on this table, assuming it is intended to
    /// be a table of successors, and returns an error if it does not make sense
    /// for a group.
    ///
    /// Invalid groups may pass these checks.
    pub fn sanity_check_successors(
        &self,
        generators: &PerGenerator<GroupElementId>,
    ) -> GroupResult<()> {
        let mut counts: PerGroupElement<usize> = (0..self.element_count).map(|_| 0).collect();

        for ((elem, generator), &successor) in self.iter() {
            let mut ok = true;

            // Applying a generator should produce a new element.
            ok &= elem != successor;

            // Only the identity has each generator as its own corresponding
            // successor.
            let is_identity = elem == GroupElementId::IDENTITY;
            ok &= is_identity == (successor == generators[generator]);

            if !ok {
                return Err(GroupError::BadGroupStructure);
            }
            counts[successor] += 1;
        }

        // Check that every element has the same number of occurrences in the
        // successor table.
        for &count in counts.iter_values() {
            if count != self.generator_count {
                return Err(GroupError::BadGroupStructure);
            }
        }

        Ok(())
    }
}
