use std::fmt;

use hypuz_util::ti::{TypedIndex, TypedIndexIter};
use itertools::Itertools;
use smallvec::{SmallVec, smallvec};

use super::{
    EggTable, GeneratorId, GroupElementId, GroupError, GroupResult, PerGenerator, PerGroupElement,
};

/// Group structure.
pub trait Group {
    /// Iterator over group elements used by [`Group::factorization()`].
    type Factorization<'a>: 'a + Iterator<Item = GeneratorId>
    where
        Self: 'a;

    /// Returns the number of elements in the group.
    fn element_count(&self) -> usize;

    /// Returns an iterator over the generators used to generate the group.
    fn generators(&self) -> &PerGenerator<GroupElementId>;
    /// Returns an iterator over the elements of the group.
    fn elements(&self) -> TypedIndexIter<GroupElementId> {
        GroupElementId::iter(self.element_count())
    }

    /// Returns the shortest factorization of `element` into generators. Ties
    /// are broken by lexicographical ordering.
    fn factorization(&self, element: GroupElementId) -> Self::Factorization<'_>;
    /// Returns the inverse of `element`.
    fn inverse(&self, element: GroupElementId) -> GroupElementId;
    /// Returns the composition of `element` and `generator`.
    fn successor(&self, element: GroupElementId, generator: GeneratorId) -> GroupElementId;
    /// Returns the composition of `element` and the inverse of `generator`.
    fn predecessor(&self, element: GroupElementId, generator: GeneratorId) -> GroupElementId;

    /// Composes two elements of the group.
    fn compose(&self, a: GroupElementId, b: GroupElementId) -> GroupElementId {
        let mut ret = a;
        for generator in self.factorization(b) {
            ret = self.successor(ret, generator);
        }
        ret
    }
}

impl<G: AsRef<AbstractGroup>> Group for G {
    type Factorization<'a>
        = std::iter::Copied<std::slice::Iter<'a, GeneratorId>>
    where
        Self: 'a;

    /// Returns an iterator over the generators used to generate the group.
    fn generators(&self) -> &PerGenerator<GroupElementId> {
        &self.as_ref().generators
    }
    /// Returns the number of elements in the group.
    fn element_count(&self) -> usize {
        self.as_ref().element_count
    }

    /// Returns the shortest factorization of `element` into generators. Ties
    /// are broken by lexicographical ordering.
    fn factorization(&self, element: GroupElementId) -> Self::Factorization<'_> {
        self.as_ref().factorizations[element].iter().copied()
    }
    /// Returns the inverse of `element`.
    fn inverse(&self, element: GroupElementId) -> GroupElementId {
        self.as_ref().inverses[element]
    }
    /// Returns the composition of `element` and `generator`.
    fn successor(&self, element: GroupElementId, generator: GeneratorId) -> GroupElementId {
        *self.as_ref().successors.get(element, generator)
    }
    /// Returns the composition of `element` and the inverse of `generator`.
    fn predecessor(&self, element: GroupElementId, generator: GeneratorId) -> GroupElementId {
        *self.as_ref().predecessors.get(element, generator)
    }
}

/// Finite group.
///
/// `ElementId(0)` is the identity.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AbstractGroup {
    /// Number of elements in the group.
    element_count: usize,
    /// Generators used to initially generate the group.
    generators: PerGenerator<GroupElementId>,
    /// Shortest generator sequence that produces each element.
    factorizations: PerGroupElement<SmallVec<[GeneratorId; 16]>>,
    /// Inverse of each element.
    inverses: PerGroupElement<GroupElementId>,
    /// Result of multiplying each element by each generator.
    successors: EggTable<GroupElementId>,
    /// Result of multiplying each element by the inverse of each generator.
    predecessors: EggTable<GroupElementId>,
}

impl Default for AbstractGroup {
    fn default() -> Self {
        AbstractGroup::new_trivial()
    }
}

impl AsRef<AbstractGroup> for AbstractGroup {
    fn as_ref(&self) -> &AbstractGroup {
        self
    }
}

impl AbstractGroup {
    /// Constructs the trivial group with no generators and only the identity
    /// element.
    pub fn new_trivial() -> Self {
        AbstractGroup {
            element_count: 1,
            generators: PerGenerator::new(),
            factorizations: std::iter::once(smallvec![]).collect(),

            inverses: std::iter::once(GroupElementId(0)).collect(),
            successors: EggTable::new(0),
            predecessors: EggTable::new(0),
        }
    }
}

/// Helper struct for constructing a group incrementally.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct GroupBuilder {
    generator_count: usize,
    element_count: usize,
    factorizations: PerGroupElement<SmallVec<[GeneratorId; 16]>>,
    successors: EggTable<Option<GroupElementId>>,
    predecessors: EggTable<Option<GroupElementId>>,
}
impl fmt::Display for GroupBuilder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "GroupBuilder {{")?;
        writeln!(f, "    generator_count: {}", self.generator_count)?;
        writeln!(f, "    element_count: {}", self.element_count)?;

        fn opt_elem_iter_to_string(iter: impl Iterator<Item = Option<GroupElementId>>) -> String {
            iter.map(|opt_elem| match opt_elem {
                Some(e) => e.to_string(),
                None => "?".to_string(),
            })
            .join(", ")
        }

        writeln!(f, "    factorizations: [")?;
        for (elem, gen_seq) in &self.factorizations {
            let factorization_string = gen_seq.iter().copied().join(", ");
            writeln!(f, "        {elem}: [{factorization_string}],")?;
        }
        writeln!(f, "    ]")?;

        writeln!(f, "    successors: [")?;
        for elem in GroupElementId::iter(self.element_count) {
            let successors_string = opt_elem_iter_to_string(
                GeneratorId::iter(self.generator_count).map(|g| self.successor(elem, g)),
            );
            writeln!(f, "        {elem}: [{successors_string}],")?;
        }
        writeln!(f, "    ]")?;

        writeln!(f, "    predecessors: [")?;
        for elem in GroupElementId::iter(self.element_count) {
            let predecessors_string = opt_elem_iter_to_string(
                GeneratorId::iter(self.generator_count).map(|g| self.predecessor(elem, g)),
            );
            writeln!(f, "        {elem}: [{predecessors_string}],")?;
        }
        writeln!(f, "    ]")?;

        writeln!(f, "}}")?;
        Ok(())
    }
}
impl GroupBuilder {
    /// Constructs a new `GroupBuilder` containing just the identity.
    pub fn new(generator_count: usize) -> GroupResult<Self> {
        // Check that there aren't too many generators.
        GeneratorId::try_from_index(generator_count)?;

        let mut factorizations = PerGroupElement::new();
        let table = EggTable::new(generator_count);
        factorizations.push(smallvec![])?; // identity has empty factorization

        Ok(GroupBuilder {
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
    pub fn build(self) -> GroupResult<AbstractGroup> {
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

        Ok(AbstractGroup {
            element_count: self.element_count,
            generators,
            factorizations: self.factorizations,
            inverses,
            successors,
            predecessors,
        })
    }
}
