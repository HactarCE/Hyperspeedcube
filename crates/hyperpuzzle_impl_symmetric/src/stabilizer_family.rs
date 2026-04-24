use std::ops::Deref;

use eyre::{OptionExt, Result, bail, eyre};
use hypergroup::{GroupAction, GroupElementId};
use hypermath::{Vector, VectorRef, approx_collections::pool::IntoIter};
use hyperpuzzle_core::{Axis, NameSpecBiMap, PerAxis, TiVec};
use itertools::Itertools;
use smallvec::SmallVec;

use crate::{NamedPoint, NamedPointSet, PerNamedPoint};

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
    /// Secondary named point set, which are setwise-stabilized by the twist.
    pub secondary: NamedPointSet,
}

impl StabilizerFamily {
    /// Returns the name for the stabilizer family, in human-friendly notation.
    pub fn name(
        &self,
        axis_names: &NameSpecBiMap<Axis>,
        named_point_names: &NameSpecBiMap<NamedPoint>,
    ) -> String {
        format!(
            "{}_{}", // TODO: proper separator
            &axis_names[self.primary],
            self.secondary.name(named_point_names),
        )
    }
}
