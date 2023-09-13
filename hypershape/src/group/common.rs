use thiserror::Error;

use hypermath::collections::{generic_vec::IndexOutOfRange, GenericVec};
use hypermath::{idx_struct, IndexNewtype, Isometry};

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
    #[error("incomplete group structure")]
    IncompleteGroupStructure,

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

/// Element-generator group table.
///
/// 2D array containing a value for each possible element+generator pairing.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(super) struct EggTable<T> {
    /// Number of elements in the group.
    element_count: usize,
    /// Number of generators in the group.
    generator_count: usize,
    /// Flattened 2D array, indexed by a pair of element ID and value index.
    contents: Vec<T>,
}
impl<T: Default + Clone> EggTable<T> {
    /// Constructs a new EGG table containing only the identity.
    pub fn new(generator_count: usize) -> Self {
        EggTable {
            element_count: 1,
            generator_count,
            contents: vec![T::default(); generator_count],
        }
    }
    /// Adds a new element to the table.
    pub fn add_element(&mut self, value: T) -> GroupResult<ElementId> {
        let new_element = ElementId::try_from_usize(self.element_count)?;
        self.element_count += 1;
        self.contents
            .extend(std::iter::repeat(value).take(self.generator_count));
        Ok(new_element)
    }
    /// Returns a value from the table.
    #[inline]
    #[track_caller]
    pub fn get(&self, element: ElementId, generator: GeneratorId) -> &T {
        &self.contents[self.index(element, generator)]
    }
    /// Returns a mutable reference to a value in the table.
    #[track_caller]
    pub fn get_mut(&mut self, element: ElementId, generator: GeneratorId) -> &mut T {
        let index = self.index(element, generator);
        &mut self.contents[index]
    }

    /// Returns an integer index into `contents`.
    #[inline]
    #[track_caller]
    fn index(&self, element: ElementId, generator: GeneratorId) -> usize {
        assert!(
            (generator.0 as usize) < self.generator_count,
            "generator {generator} out of range (max {max})",
            max = self.generator_count,
        );
        element.0 as usize * self.generator_count + generator.0 as usize
    }

    /// Returns an iterator over keys and values.
    pub fn iter(&self) -> impl '_ + Iterator<Item = ((ElementId, GeneratorId), &T)> {
        let elements_iter = ElementId::iter(self.element_count);
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
impl EggTable<ElementId> {
    /// Performs basic sanity checks on this table, assuming it is intended to
    /// be a table of successors, and returns an error if it does not make sense
    /// for a group.
    ///
    /// Invalid groups may pass these checks.
    pub fn sanity_check_successors(&self) -> GroupResult<()> {
        let mut counts: PerElement<usize> = (0..self.element_count).map(|_| 0).collect();

        for ((elem, gen), &successor) in self.iter() {
            let mut ok = true;

            // Applying a generator should produce a new element.
            ok &= elem != successor;

            // Only the identity has each generator as its own corresponding
            // successor.
            let is_identity = elem == ElementId::IDENTITY;
            ok &= is_identity == (successor == ElementId::from(gen));

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
