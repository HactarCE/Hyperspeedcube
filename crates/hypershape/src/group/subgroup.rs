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
    /// as the group and contains all the elements of the group.
    pub fn new_total(group: Arc<AbstractGroup>) -> Self {
        let generators = group.generators().map(|g| g.into()).collect();
        let elements = TiMask::new_full(group.element_count());
        Self {
            group,
            generators,
            elements,
        }
    }

    /// Generates a subgroup from generators.
    pub fn new(group: Arc<AbstractGroup>, generators: &[GroupElementId]) -> Self {
        let mut ret = Self::new_trivial(group);
        ret.add_generators(generators);
        ret
    }

    /// Adds generators to the subgroup and discovers new elements.
    pub fn add_generators(&mut self, new_generators: &[GroupElementId]) {
        self.generators.extend_from_slice(new_generators);
        let old_elements = self.elements.clone();
        for &new_generator in new_generators {
            for old_elem in &old_elements {
                let init = self.group.compose(old_elem, new_generator);
                if !self.elements.contains(init) {
                    self.elements.insert(init);
                    super::orbit(init, &self.generators, |&e, &g| {
                        let new_elem = self.group.compose(e, g);
                        (!self.elements.contains(new_elem)).then(|| {
                            self.elements.insert(new_elem);
                            new_elem
                        })
                    });
                }
            }
        }
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

/// Coset of a subgroup in a group.
///
/// This represents either a left coset or a right coset, depending on the order
/// of composition between `offset` and an element of `subgroup`.
#[derive(Debug, Copy, Clone)]
pub struct Coset<'a> {
    /// Subgroup of the coset.
    pub subgroup: &'a Subgroup,
    /// Offset of the coset.
    pub offset: GroupElementId,
}
