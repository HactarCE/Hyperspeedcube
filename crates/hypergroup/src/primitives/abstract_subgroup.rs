use std::{fmt, sync::Arc};

use hypuz_util::ti::TiMask;

use super::AbstractGroupLut;

use crate::GroupElementId;

/// Subgroup of an [`AbstractGroupLut`].
pub(crate) struct AbstractSubgroup {
    /// Group that this is a subgroup of.
    overgroup: Arc<AbstractGroupLut>,
    /// Generating set for the subgroup.
    generators: Vec<GroupElementId>,
    /// Number of elements in the subgroup.
    element_count: usize,
    /// Subset of elements from `overgroup` that are in the subgroup.
    element_subset: TiMask<GroupElementId>,
}

impl fmt::Debug for AbstractSubgroup {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AbstractSubgroup")
            .field("overgroup", &self.overgroup)
            .field("generators", &self.generators)
            .field("element_count", &self.element_count)
            .field("element_subset", &self.element_subset)
            .finish()
    }
}

impl AbstractSubgroup {
    /// Constructs the trivial subgroup of a group, which has no generators and
    /// contains only the identity element.
    pub fn new_trivial(overgroup: Arc<AbstractGroupLut>) -> Self {
        let element_subset =
            TiMask::from_element(overgroup.element_count(), GroupElementId::IDENTITY);
        Self {
            overgroup,
            generators: vec![],
            element_count: 1,
            element_subset,
        }
    }

    /// Constructs the total subgroup of a group, which has the same generators
    /// as the group and contains all the elements of the original group.
    pub fn new_total(overgroup: Arc<AbstractGroupLut>) -> Self {
        let generators = overgroup.generators().iter_values().copied().collect();
        let element_count = overgroup.element_count();
        let element_subset =
            TiMask::from_element(overgroup.element_count(), GroupElementId::IDENTITY);
        Self {
            overgroup,
            generators,
            element_count,
            element_subset,
        }
    }

    /// Generates a subgroup from generators.
    pub fn new(overgroup: Arc<AbstractGroupLut>, generators: Vec<GroupElementId>) -> Self {
        let mut ret = Self::new_trivial(overgroup);
        ret.generators = generators;
        crate::orbit(GroupElementId::IDENTITY, &ret.generators, |g, e| {
            let new_elem = ret.overgroup.compose(*e, *g);
            (!ret.element_subset.contains(new_elem)).then(|| {
                ret.element_count += 1;
                ret.element_subset.insert(new_elem);
                new_elem
            })
        });
        ret
    }

    /// Returns the group that this is a subgroup of.
    pub fn overgroup(&self) -> &Arc<AbstractGroupLut> {
        &self.overgroup
    }

    /// Returns whether the subgroup is trivial (contains only the identity).
    pub fn is_trivial(&self) -> bool {
        self.element_count == 1
    }

    /// Returns a canonical generating set for the subgroup.
    pub fn generators(&self) -> &[GroupElementId] {
        &self.generators
    }

    /// Returns the number of elements in the subgroup.
    pub fn element_count(&self) -> usize {
        self.element_count
    }

    /// Returns a mask indicating the subset of elements in the subgroup from
    /// the overgroup.
    pub fn elements(&self) -> &TiMask<GroupElementId> {
        &self.element_subset
    }

    /// Returns whether `element` is in the subgroup.
    pub fn contains(&self, element: GroupElementId) -> bool {
        self.element_subset.contains(element)
    }
}
