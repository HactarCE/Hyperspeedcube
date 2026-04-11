use std::ops::Deref;

use eyre::{OptionExt, Result, bail, eyre};
use hypergroup::{GroupAction, GroupElementId};
use hypermath::{Vector, VectorRef};
use hyperpuzzle_core::{Axis, PerAxis, TiVec};
use itertools::Itertools;
use smallvec::SmallVec;

/// Twist [family] that consists of an axis to twist (the "primary" axis)
/// followed by a set of axes (the "secondary" axes) that is setwise-stabilized
/// by the rotation.
///
/// This is commonly used for 4D puzzles as a compact alternative to [constraint
/// notation]. In this case, the **primary vector** is defined as the axis
/// vector of the primary axis and the **secondary vector** is defined as the
/// sum of the axis vectors of the secondary axes. The twist is then a clockwise
/// rotation around the plane spanned by the primary vector and the secondary
/// vector.
///
/// [family]: hypuz_notation::Transform::family
/// [constraint notation]: hypuz_notation::Transform::constraints
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct StabilizerFamily {
    /// Primary axis, which is twisted.
    pub primary: Axis,
    /// Secondary axes, which are setwise-stabilized by the twist.
    pub secondary: StabilizableAxisSet,
}

impl StabilizerFamily {
    /// Returns an iterator over all the axes, including the primary axes and
    /// the secondary axes.
    pub(crate) fn iter_flatten(&self) -> impl Iterator<Item = Axis> {
        std::iter::chain([self.primary], self.secondary.0.iter().copied())
    }

    /// Returns the set of constraints implied by the stabilizer family.
    ///
    /// This set of constraints should be satisfied by a cyclic subgroup, whose
    /// minimal clockwise generator is the unit twist transform for this twist
    /// family.
    pub(crate) fn constraint_set(&self) -> Result<hypergroup::ConstraintSet<Axis>> {
        if self.secondary.len() > 3 {
            bail!(
                "cannot compute unit twist transform for more than 3 axes; this is a program limitation",
            );
        }
        Ok(hypergroup::ConstraintSet::from_iter(std::iter::chain(
            [hypergroup::Constraint::fix(self.primary)],
            self.secondary
                .0
                .iter()
                .circular_tuple_windows()
                .map(|(&from, &to)| hypergroup::Constraint { from, to }),
        )))
    }
}

/// Small list of axes which are setwise-stabilized by some nontrivial subgroup
/// of the grip group. This is used to define [`StabilizerFamily`] twists.
///
/// All permutations of the list are assumed to be reachable using the action of
/// the grip group.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct StabilizableAxisSet(SmallVec<[Axis; 3]>);

impl StabilizableAxisSet {
    /// Constructs a list of axis.
    pub fn new(axes: SmallVec<[Axis; 3]>) -> Result<Self> {
        if axes.len() > 3 {
            bail!(
                "pseudo-axes with more than 3 axes are not supported; please contact the developer for more info"
            );
        }
        Ok(Self(axes))
    }

    pub fn transform_by_group_element(
        &self,
        action: &GroupAction<Axis>,
        element: GroupElementId,
    ) -> Self {
        Self(self.0.iter().map(|&a| action.act(element, a)).collect())
    }

    /// Offsets all axis IDs by some amount.
    pub(crate) fn offset_ids_by(&self, id_offset: usize) -> Self {
        Self(
            self.0
                .iter()
                .map(|&a| Axis(a.0 + id_offset as u16))
                .collect(),
        )
    }

    /// Returns the number of axes in the list.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns whether the list is empty.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Returns a vector between all of the axes.
    ///
    /// This vector's stabilizer is presumed to be the setwise stabilizer of the
    /// set of axes.
    pub fn vector(&self, axis_vectors: &PerAxis<Vector>) -> Vector {
        self.0.iter().map(|&a| &axis_vectors[a]).sum()
    }
}
