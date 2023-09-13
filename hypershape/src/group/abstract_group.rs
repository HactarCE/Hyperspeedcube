use std::fmt;

use itertools::Itertools;
use smallvec::{smallvec, SmallVec};

use super::{EggTable, ElementId, GeneratorId, GroupError, GroupResult, PerElement, PerGenerator};
use hypermath::{collections::generic_vec::IndexIter, IndexNewtype};

/// Group structure.
pub trait Group {
    /// Returns the underlying abstract group.
    fn group(&self) -> &AbstractGroup;

    /// Returns the number of generators used to generate the group.
    fn generator_count(&self) -> usize {
        self.group().generator_count
    }
    /// Returns the number of elements in the group.
    fn element_count(&self) -> usize {
        self.group().element_count
    }

    /// Returns an iterator over the generators used to generate the group.
    fn generators(&self) -> IndexIter<GeneratorId> {
        GeneratorId::iter(self.generator_count())
    }
    /// Returns an iterator over the elements of the group.
    fn elements(&self) -> IndexIter<ElementId> {
        ElementId::iter(self.element_count())
    }

    /// Returns the shortest factorization of `element` into generators. Ties
    /// are broken by lexicographical ordering.
    fn factorization(&self, element: ElementId) -> &[GeneratorId] {
        &self.group().factorizations[element]
    }
    /// Returns the inverse of `element`.
    fn inverse(&self, element: ElementId) -> ElementId {
        self.group().inverses[element]
    }
    /// Returns the composition of `element` and `generator`.
    fn successor(&self, element: ElementId, generator: GeneratorId) -> ElementId {
        *self.group().successors.get(element, generator)
    }
    /// Returns the composition of `element` and the inverse of `generator`.
    fn predecessor(&self, element: ElementId, generator: GeneratorId) -> ElementId {
        *self.group().predecessors.get(element, generator)
    }

    /// Composes two elements of the group.
    fn compose(&self, a: ElementId, b: ElementId) -> ElementId {
        let mut ret = a;
        for &gen in self.factorization(b) {
            ret = self.successor(ret, gen);
        }
        ret
    }
}

/// Finite group.
///
/// `ElementId(0)` is the identity. `ElementId(n)` where _1 ≤ n ≤ N_ are the
/// generators (where _N_ is the number of generators).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AbstractGroup {
    /// Number of generators used to initially generate the group. If there are
    /// `N` generators, then the generators are always elements `1..=N`.
    generator_count: usize,
    /// Number of elements in the group.
    element_count: usize,

    /// Shortest generator sequence that produces each element.
    factorizations: PerElement<SmallVec<[GeneratorId; 16]>>,
    /// Inverse of each element.
    inverses: PerElement<ElementId>,
    /// Result of multiplying each element by each generator.
    successors: EggTable<ElementId>,
    /// Result of multiplying each element by the inverse of each generator.
    predecessors: EggTable<ElementId>,
}

impl Default for AbstractGroup {
    fn default() -> Self {
        AbstractGroup::new_trivial()
    }
}

impl Group for AbstractGroup {
    fn group(&self) -> &AbstractGroup {
        self
    }
}

impl AbstractGroup {
    /// Constructs the trivial group with no generators and only the identity
    /// element.
    pub fn new_trivial() -> Self {
        AbstractGroup {
            generator_count: 0,
            element_count: 1,
            factorizations: std::iter::once(smallvec![]).collect(),

            inverses: std::iter::once(ElementId(0)).collect(),
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

    generator_inverses: PerGenerator<Option<ElementId>>,

    factorizations: PerElement<SmallVec<[GeneratorId; 16]>>,
    successors: EggTable<Option<ElementId>>,
    predecessors: EggTable<Option<ElementId>>,
}
impl fmt::Display for GroupBuilder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "GroupBuilder {{")?;
        writeln!(f, "    generator_count: {}", self.generator_count)?;
        writeln!(f, "    element_count: {}", self.element_count)?;

        fn opt_elem_iter_to_string(iter: impl Iterator<Item = Option<ElementId>>) -> String {
            iter.map(|opt_elem| match opt_elem {
                Some(e) => e.to_string(),
                None => "?".to_string(),
            })
            .join(", ")
        }

        let generator_inverses_string =
            opt_elem_iter_to_string(self.generator_inverses.iter_values().copied());
        writeln!(f, "    generator_inverses: [{generator_inverses_string}]")?;

        writeln!(f, "    factorizations: [")?;
        for (elem, gen_seq) in &self.factorizations {
            let factorization_string = gen_seq.iter().copied().map(ElementId::from).join(", ");
            writeln!(f, "        {elem}: [{factorization_string}],")?;
        }
        writeln!(f, "    ]")?;

        writeln!(f, "    successors: [")?;
        for elem in ElementId::iter(self.element_count) {
            let successors_string = opt_elem_iter_to_string(
                GeneratorId::iter(self.generator_count).map(|gen| self.successor(elem, gen)),
            );
            writeln!(f, "        {elem}: [{successors_string}],")?;
        }
        writeln!(f, "    ]")?;

        writeln!(f, "    predecessors: [")?;
        for elem in ElementId::iter(self.element_count) {
            let predecessors_string = opt_elem_iter_to_string(
                GeneratorId::iter(self.generator_count).map(|gen| self.predecessor(elem, gen)),
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
        GeneratorId::try_from_usize(generator_count)?;

        let mut factorizations = PerElement::new();
        let table = EggTable::new(generator_count);
        factorizations.push(smallvec![])?; // identity has empty factorization

        Ok(GroupBuilder {
            generator_count,
            element_count: 1,

            generator_inverses: (0..generator_count).map(|_| None).collect(),

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
        element: ElementId,
        generator: GeneratorId,
    ) -> GroupResult<ElementId> {
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
        element: ElementId,
        generator: GeneratorId,
    ) -> GroupResult<ElementId> {
        self.element_count += 1;

        let mut factorization = self.factorization(element).clone();
        factorization.push(generator);
        let new_element = self.factorizations.push(factorization)?;

        // We don't yet know its successors.
        self.successors.add_element(None)?;
        self.predecessors.add_element(None)?;

        // The new element is a successor of the old one.
        self.set_successor(element, generator, new_element);

        Ok(new_element)
    }
    /// Sets the composition of `element * generator` to `result`. Also sets the
    /// predecessor: `result * generator^(-1)`. Returns `true` if the relation
    /// was previously unknown.
    pub fn set_successor(
        &mut self,
        element: ElementId,
        generator: GeneratorId,
        result: ElementId,
    ) -> bool {
        let is_new = self.successor(element, generator).is_none();

        *self.successors.get_mut(element, generator) = Some(result);
        *self.predecessors.get_mut(result, generator) = Some(element);

        if result == ElementId::IDENTITY {
            // `element * generator = 1`, so we found the inverse of `generator`.
            self.generator_inverses[generator] = Some(element);
        }

        is_new
    }

    /// Returns the number of generators in the group. This cannot be changed
    /// after constructing the [`GroupBuilder`].
    pub fn generator_count(&self) -> usize {
        self.generator_count
    }
    /// Returns the number of known elements in the group. This can only
    /// increase as more elements are discovered.
    pub fn element_count(&self) -> usize {
        self.element_count
    }

    /// Returns the factorization of an element into generators.
    pub fn factorization(&self, element: ElementId) -> &SmallVec<[GeneratorId; 16]> {
        &self.factorizations[element]
    }
    /// Returns the result of `element * generator`, or `None` if it is unknown.
    pub fn successor(&self, element: ElementId, generator: GeneratorId) -> Option<ElementId> {
        *self.successors.get(element, generator)
    }
    /// Returns the result of `element * generator^(-1)`, or `None` if it is
    /// unknown.
    pub fn predecessor(&self, element: ElementId, generator: GeneratorId) -> Option<ElementId> {
        *self.predecessors.get(element, generator)
    }

    /// Constructs a group, returning an error if some basic sanity checks fail.
    pub fn build(self) -> GroupResult<AbstractGroup> {
        let successors = self.successors.try_unwrap()?;
        let predecessors = self.predecessors.try_unwrap()?;
        successors.sanity_check_successors()?;

        let inverses = self
            .factorizations
            .iter_values()
            .map(|factorization| {
                factorization
                    .iter()
                    .rev()
                    .fold(ElementId::IDENTITY, |elem, &gen| {
                        *predecessors.get(elem, gen)
                    })
            })
            .collect::<PerElement<ElementId>>();

        // Check that the inverse property holds.
        for (elem, &inverse) in &inverses {
            let inverse_of_inverse = inverses[inverse];
            if elem != inverse_of_inverse {
                return Err(GroupError::BadInverse(elem, inverse, inverse_of_inverse));
            }
        }

        Ok(AbstractGroup {
            generator_count: self.generator_count,
            element_count: self.element_count,

            factorizations: self.factorizations,
            inverses,
            successors,
            predecessors,
        })
    }
}
