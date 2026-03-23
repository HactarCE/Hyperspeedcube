use std::{borrow::Cow, fmt};

use hypuz_util::ti::{TypedIndex, TypedIndexIter};
use smallvec::{SmallVec, smallvec};

use super::EggTable;
use crate::{GeneratorId, GroupElementId, GroupResult, PerGenerator, PerGroupElement};

/// Lookup tables for an [abstract] [finite group].
///
/// [abstract]: https://en.wikipedia.org/wiki/Group_theory#Abstract_groups
/// [identity]: https://en.wikipedia.org/wiki/Identity_element
#[derive(Clone)]
pub(crate) struct AbstractGroupLut {
    /// Name for the group, used in debug printing and other diagnostics.
    pub(super) label: Cow<'static, str>,

    /// Generators used to initially generate the group.
    pub(super) generators: PerGenerator<GroupElementId>,
    /// Shortest generator sequence that produces each element.
    pub(super) factorizations: PerGroupElement<SmallVec<[GeneratorId; 16]>>,
    /// Inverse of each element.
    pub(super) inverses: PerGroupElement<GroupElementId>,
    /// Result of multiplying each element by each generator.
    pub(super) successors: EggTable<GroupElementId>,
}

impl Default for AbstractGroupLut {
    fn default() -> Self {
        AbstractGroupLut::new_trivial()
    }
}

impl fmt::Debug for AbstractGroupLut {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AbstractGroupLut")
            .field("label", &self.label)
            .field("generators", &self.generators)
            .field("element_count", &self.element_count())
            .finish_non_exhaustive()
    }
}

impl AbstractGroupLut {
    /// Constructs the trivial group with no generators and only the identity
    /// element.
    pub fn new_trivial() -> Self {
        AbstractGroupLut {
            label: "trivial".into(),

            generators: PerGenerator::new(),
            factorizations: std::iter::once(smallvec![]).collect(),

            inverses: std::iter::once(GroupElementId(0)).collect(),
            successors: EggTable::new(0),
        }
    }

    /// Returns the group's label
    pub fn label(&self) -> &str {
        &self.label
    }

    /// Constructs a group from a composition function.
    ///
    /// The composition function is called on every pair of group element and
    /// generator, in order. It must return either an existing element, or an
    /// element that is one more than the largest element that has been returned
    /// previously.
    pub fn from_compose_fn(
        label: impl Into<Cow<'static, str>>,
        generator_count: usize,
        mut compose: impl FnMut(GroupElementId, GeneratorId) -> GroupResult<GroupElementId>,
    ) -> GroupResult<Self> {
        // TODO: allow adding elements in any order. deal with it.

        let mut builder = crate::primitives::AbstractGroupLutBuilder::new(label, generator_count)?;
        let mut e = GroupElementId::IDENTITY;
        while (e.0 as usize) < builder.element_count() {
            for g in GeneratorId::iter(generator_count) {
                let result = compose(e, g)?;
                if (result.0 as usize) < builder.element_count() {
                    builder.set_successor(e, g, result);
                } else {
                    assert_eq!(
                        result,
                        builder.add_successor(e, g)?,
                        "elements returned from `compose` must not skip",
                    );
                }
            }
            e = e.next()?;
        }
        builder.build()
    }

    /// Returns the list of generators used to generate the group.
    pub fn generators(&self) -> &PerGenerator<GroupElementId> {
        &self.generators
    }

    /// Returns the number of elements in the group.
    pub fn element_count(&self) -> usize {
        self.factorizations.len()
    }

    /// Returns an iterator over the elements in the group.
    pub fn elements(&self) -> TypedIndexIter<GroupElementId> {
        GroupElementId::iter(self.element_count())
    }

    /// Returns the shortest factorization of `element` into generators. Ties
    /// are broken by lexicographical ordering.
    pub fn factorization(&self, element: GroupElementId) -> &[GeneratorId] {
        &self.factorizations[element]
    }

    /// Returns the inverse of `element`.
    pub fn inverse(&self, element: GroupElementId) -> GroupElementId {
        self.inverses[element]
    }

    /// Composes an element and a generator.
    pub fn compose_elem_generator(&self, e: GroupElementId, g: GeneratorId) -> GroupElementId {
        *self.successors.get(e, g)
    }

    /// Composes two elements of the group.
    pub fn compose(&self, a: GroupElementId, b: GroupElementId) -> GroupElementId {
        let mut ret = a;
        for &generator in self.factorization(b) {
            ret = self.compose_elem_generator(ret, generator);
        }
        ret
    }

    /// Conjugates two elements of the group.
    pub fn conjugate(&self, a: GroupElementId, b: GroupElementId) -> GroupElementId {
        self.compose(self.compose(a, b), self.inverse(a))
    }
}
