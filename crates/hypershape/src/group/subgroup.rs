use std::sync::Arc;

use hypuz_util::ti::TiMask;

use super::{AbstractGroup, Group, GroupElementId};

/// Subgroup of a group.
#[derive(Debug)]
pub struct Subgroup {
    /// Group that this is a subgroup of.
    group: Arc<AbstractGroup>,
    /// Generating set for the subgroup.
    ///
    /// This is not necessarily minimal.
    generators: Vec<GroupElementId>,
    /// Mask of elements in the subgroup.
    elements: TiMask<GroupElementId>,
}

impl Subgroup {
    /// Constructs the trivial subgroup of a group, which has no generators and
    /// contains only the identity element.
    pub fn new_trivial(group: Arc<AbstractGroup>) -> Self {
        let elements = TiMask::from_iter(group.element_count(), [GroupElementId::IDENTITY]);
        Self {
            group,
            generators: vec![],
            elements,
        }
    }

    /// Returns whether the subgroup is trivial (contains only the identity).
    pub fn is_trivial(&self) -> bool {
        self.generators.is_empty()
    }

    /// Constructs the total subgroup of a group, which has the same generators
    /// as the group and contains all the elements of the original group.
    pub fn new_total(group: Arc<AbstractGroup>) -> Self {
        let generators = group.generators().map(|g| g.into()).collect();
        let elements = TiMask::new_full(group.element_count());
        Self {
            group,
            generators,
            elements,
        }
    }

    /// Returns the elements in the subgroup, as a subset of the original group.
    pub fn elements(&self) -> &TiMask<GroupElementId> {
        &self.elements
    }

    /// Generates a subgroup from generators.
    pub fn new(group: Arc<AbstractGroup>, generators: Vec<GroupElementId>) -> Self {
        let mut ret = Self::new_trivial(group);
        ret.generators = generators;
        crate::orbit(GroupElementId::IDENTITY, &ret.generators, |&e, &g| {
            let new_elem = ret.group.compose(e, g);
            if !ret.elements.contains(new_elem) {
                ret.elements.insert(new_elem);
                Some(new_elem)
            } else {
                None
            }
        });
        ret
    }

    /// Returns the group that this is a subgroup of.
    pub fn overgroup(&self) -> &AbstractGroup {
        &self.group
    }

    /// Returns the number of elements in the subgroup.
    pub fn element_count(&self) -> usize {
        self.elements.len()
    }

    /// Returns a generating set for the subgroup.
    pub fn generating_set(&self) -> &[GroupElementId] {
        &self.generators
    }
}

/// Sandwich of a subgroup by group elements: `lhs * subgroup * rhs`.
///
/// This is _not_ a coset of the subgroup, but it can be written as any of the
/// following:
///
/// - Left [coset] of a conjugated subgroup: `(lhs * rhs) * (rhs^1 * subgroup *
///   rhs)`
/// - Right [coset] of a conjugated subgroup: `(lhs * subgroup * lhs^1) * (lhs *
///   rhs)`
/// - Conjugate of a left [coset]: `rhs^1 * ((rhs * lhs) * subgroup) * rhs`
/// - Conjugate of a right [coset]: `lhs * (subgroup * (rhs * lhs)) * lhs^1`
///
/// This is **not** the same thing as a [double coset].
///
/// [coset]: https://en.wikipedia.org/wiki/Coset
/// [double coset]: https://en.wikipedia.org/wiki/Double_coset
#[derive(Debug, Copy, Clone)]
pub struct ConjugateCoset<'a> {
    /// Element to multiply on the left of the subgroup.
    pub lhs: GroupElementId,
    /// Subgroup.
    pub subgroup: &'a Subgroup,
    /// Element to multiple on the right of the subgroup.
    pub rhs: GroupElementId,
}
