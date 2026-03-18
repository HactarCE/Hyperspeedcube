use std::borrow::Cow;

use hypuz_util::ti::TypedIndex;
use smallvec::{SmallVec, smallvec};

use super::{AbstractGroupLut, EggTable};
use crate::{GeneratorId, GroupElementId, GroupError, GroupResult, PerGroupElement};

/// Helper struct for constructing an [`AbstractGroupLut`] incrementally.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct AbstractGroupLutBuilder {
    pub label: Cow<'static, str>,

    generator_count: usize,
    element_count: usize,
    factorizations: PerGroupElement<SmallVec<[GeneratorId; 16]>>,
    successors: EggTable<Option<GroupElementId>>,
    predecessors: EggTable<Option<GroupElementId>>,
}

impl AbstractGroupLutBuilder {
    /// Constructs a new `GroupBuilder` containing just the identity.
    pub fn new(label: impl Into<Cow<'static, str>>, generator_count: usize) -> GroupResult<Self> {
        // Check that there aren't too many generators.
        GeneratorId::try_from_index(generator_count)?;

        let mut factorizations = PerGroupElement::new();
        let table = EggTable::new(generator_count);
        factorizations.push(smallvec![])?; // identity has empty factorization

        Ok(AbstractGroupLutBuilder {
            label: label.into(),
            generator_count,
            element_count: 1,
            factorizations,
            successors: table.clone(),
            predecessors: table,
        })
    }

    /// Returns the composition of `element * generator`, adding a new element
    /// to the group (by calling [`GroupBuilder::add_successor()`]) if it is
    /// unknown.
    pub fn get_or_add_successor(
        &mut self,
        element: GroupElementId,
        generator: GeneratorId,
    ) -> GroupResult<GroupElementId> {
        match self.successor(element, generator) {
            Some(existing_element) => Ok(existing_element),
            None => self.add_successor(element, generator),
        }
    }
    /// Adds a new element and sets the composition of `element * generator` to
    /// that new element using [`GroupBuilder::set_successor()`]. Also sets the
    /// predecessor of the new element.
    pub fn add_successor(
        &mut self,
        element: GroupElementId,
        generator: GeneratorId,
    ) -> GroupResult<GroupElementId> {
        self.element_count += 1;

        let mut factorization = self.factorization(element).clone();
        factorization.push(generator);
        let new_element = self.factorizations.push(factorization)?;

        // We don't yet know its successors.
        self.successors.add_element()?;
        self.predecessors.add_element()?;

        // The new element is a successor of the old one.
        self.set_successor(element, generator, new_element);

        Ok(new_element)
    }
    /// Sets the composition of `element * generator` to `result`. Also sets the
    /// predecessor: `result * generator^(-1)`. Returns `true` if the relation
    /// was previously unknown.
    pub fn set_successor(
        &mut self,
        element: GroupElementId,
        generator: GeneratorId,
        result: GroupElementId,
    ) -> bool {
        let is_new = self.successor(element, generator).is_none();

        *self.successors.get_mut(element, generator) = Some(result);
        *self.predecessors.get_mut(result, generator) = Some(element);

        is_new
    }

    /// Returns the number of known elements in the group. This can only
    /// increase as more elements are discovered.
    pub fn element_count(&self) -> usize {
        self.element_count
    }

    /// Returns the factorization of an element into generators.
    pub fn factorization(&self, element: GroupElementId) -> &SmallVec<[GeneratorId; 16]> {
        &self.factorizations[element]
    }
    /// Returns the result of `element * generator`, or `None` if it is unknown.
    pub fn successor(
        &self,
        element: GroupElementId,
        generator: GeneratorId,
    ) -> Option<GroupElementId> {
        *self.successors.get(element, generator)
    }
    /// Returns the result of `element * generator^(-1)`, or `None` if it is
    /// unknown.
    pub fn predecessor(
        &self,
        element: GroupElementId,
        generator: GeneratorId,
    ) -> Option<GroupElementId> {
        *self.predecessors.get(element, generator)
    }

    /// Constructs a group, returning an error if some basic sanity checks fail.
    pub fn build(self) -> GroupResult<AbstractGroupLut> {
        let successors = self.successors.try_unwrap()?;
        let predecessors = self.predecessors.try_unwrap()?;

        let generators = GeneratorId::iter(self.generator_count)
            .map(|g| *successors.get(GroupElementId::IDENTITY, g))
            .collect();

        successors.sanity_check_successors(&generators)?;

        let inverses = self
            .factorizations
            .iter_values()
            .map(|factorization| {
                factorization
                    .iter()
                    .rev()
                    .fold(GroupElementId::IDENTITY, |elem, &generator| {
                        *predecessors.get(elem, generator)
                    })
            })
            .collect::<PerGroupElement<GroupElementId>>();

        // Check that the inverse property holds.
        for (elem, &inverse) in &inverses {
            let inverse_of_inverse = inverses[inverse];
            if elem != inverse_of_inverse {
                return Err(GroupError::BadInverse(elem, inverse, inverse_of_inverse));
            }
        }

        Ok(AbstractGroupLut {
            label: self.label,
            generators: (1..=self.generator_count as u32)
                .map(GroupElementId)
                .collect(),
            factorizations: self.factorizations,
            inverses,
            successors,
            predecessors,
        })
    }
}
