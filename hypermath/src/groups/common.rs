use thiserror::Error;

use crate::{
    collections::{generic_vec::IndexOutOfRange, GenericVec},
    IndexNewtype, Isometry,
};

idx_struct! {
    /// ID of a group element.
    pub struct GeneratorId(pub(super) u8);
    /// ID of a group element.
    pub struct ElementId(pub(super) u16);
}
impl From<GeneratorId> for ElementId {
    fn from(value: GeneratorId) -> Self {
        ElementId(value.0 as u16 + 1)
    }
}
impl ElementId {
    /// Identity element in any group.
    pub const IDENTITY: ElementId = ElementId(0);
}

/// List containing a value per group generator.
pub type PerGenerator<T> = GenericVec<GeneratorId, T>;
/// List containing a value per group element.
pub type PerElement<T> = GenericVec<ElementId, T>;

/// Error that can occur during group construction.
#[allow(missing_docs)]
#[derive(Error, Debug, Clone)]
pub enum GroupError {
    #[error("invalid group generator {0}")]
    InvalidGenerator(Isometry),
    #[error("overflow ({0})")]
    Overflow(IndexOutOfRange),

    #[error("missing inverse for element {0}")]
    MissingInverse(ElementId),
    #[error("missing successor for element {0} and generator {1}")]
    MissingSuccessor(ElementId, GeneratorId),

    #[error("bad group structure")]
    BadGroupStructure,
    #[error("bad inverse; inverse of {0} is {1} but inverse of {1} is {2}")]
    BadInverse(ElementId, ElementId, ElementId),
}
impl From<IndexOutOfRange> for GroupError {
    fn from(value: IndexOutOfRange) -> Self {
        GroupError::Overflow(value)
    }
}

/// Result type returned by group construction operations.
pub type GroupResult<T> = Result<T, GroupError>;

/// 2D array containing a fixed number of values for each element in the group.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(super) struct ArrayPerElement {
    /// Number of elements in the group.
    element_count: usize,
    /// Number of values per element.
    values_per_element: usize,
    /// Flattened 2D array, indexed by a pair of element ID and value index.
    contents: Vec<ElementId>,
}
impl ArrayPerElement {
    /// Constructs a new successor table containing only the identity.
    pub fn new(value_count: usize) -> GroupResult<Self> {
        let mut ret = ArrayPerElement {
            element_count: 0,
            values_per_element: value_count,
            contents: vec![],
        };
        ret.add_element()?; // Add the identity.
        Ok(ret)
    }
    /// Adds a new element to the table. By default, all values of are the new
    /// element.
    pub fn add_element(&mut self) -> GroupResult<ElementId> {
        let new_element = ElementId::try_from_usize(self.element_count)?;
        self.element_count += 1;
        self.contents
            .extend(std::iter::repeat(new_element).take(self.values_per_element));
        Ok(new_element)
    }
    /// Returns a value from the table.
    #[inline]
    pub fn get(&self, element: ElementId, value_index: usize) -> ElementId {
        self.contents[self.index(element, value_index)]
    }
    /// Returns a value from the table, or `None` if it is still default.
    #[inline]
    pub fn get_non_default(&self, element: ElementId, value_index: usize) -> Option<ElementId> {
        let result = self.get(element, value_index);
        (result != element).then_some(result)
    }
    /// Returns a mutable reference to a value in the table.
    pub fn get_mut(&mut self, element: ElementId, value_index: usize) -> &mut ElementId {
        let index = self.index(element, value_index);
        &mut self.contents[index]
    }

    /// Returns an integer index into `contents`.
    #[inline]
    fn index(&self, element: ElementId, value_index: usize) -> usize {
        element.0 as usize * self.values_per_element + value_index
    }

    /// Returns an iterator over keys and values.
    fn iter(&self) -> impl '_ + Iterator<Item = ((ElementId, usize), ElementId)> {
        let elements_iter = ElementId::iter(self.element_count);
        let generators_iter = 0..self.values_per_element;
        itertools::iproduct!(elements_iter, generators_iter).zip(self.contents.iter().copied())
    }

    /// Performs basic sanity checks on this table, assuming it is intended to
    /// be a table of successors, and returns an error if it does not make sense
    /// for a group.
    ///
    /// Invalid groups may pass these checks.
    pub fn sanity_check_successors(&self) -> GroupResult<()> {
        let mut counts: PerElement<usize> = (0..self.element_count).map(|_| 0).collect();

        for ((elem, index), successor) in self.iter() {
            let mut ok = true;

            // Applying a generator should produce a new element.
            ok &= elem != successor;

            // Only the identity has each generator as its own corresponding
            // successor.
            let is_identity = elem == ElementId::IDENTITY;
            let generator = GeneratorId::try_from_usize(index)?;
            ok &= is_identity == (successor == ElementId::from(generator));

            if !ok {
                return Err(GroupError::BadGroupStructure);
            }
            counts[successor] += 1;
        }

        // Check that every element has the same number of occurrences in the
        // successor table.
        for &count in counts.iter_values() {
            if count != self.values_per_element {
                return Err(GroupError::BadGroupStructure);
            }
        }

        Ok(())
    }
}
