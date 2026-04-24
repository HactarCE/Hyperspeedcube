use std::ops::Deref;

use eyre::{OptionExt, Result, bail, eyre};
use hypergroup::{GroupAction, GroupElementId};
use hypermath::{Vector, VectorRef, approx_collections::pool::IntoIter};
use hyperpuzzle_core::{Axis, NameSpecBiMap, PerAxis, TiVec};
use itertools::Itertools;
use smallvec::SmallVec;

hypuz_util::typed_index_struct! {
    /// Named point, which is used for describing twists and rotations.
    pub struct NamedPoint(pub u16);
}

/// List containing a value per named point.
pub type PerNamedPoint<T> = TiVec<NamedPoint, T>;

/// Small list of named points which are setwise-stabilized by some nontrivial
/// subgroup of the grip group. This is used to define [`StabilizerFamily`]
/// twists. This list may be empty (and often is in 3D).
///
/// All permutations of the list are assumed to be reachable using the action of
/// the grip group.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NamedPointSet(SmallVec<[NamedPoint; 3]>);

impl NamedPointSet {
    /// Empty set.
    pub const EMPTY: Self = NamedPointSet(SmallVec::new_const());

    /// Constructs a list of named points.
    pub fn new(named_points: SmallVec<[NamedPoint; 3]>) -> Result<Self> {
        if named_points.len() > 3 {
            bail!(
                "named point sets with more than 3 axes are not supported; \
                 please contact the developer for more info",
            );
        }
        Ok(Self(named_points))
    }

    /// Transforms each named point in a set by a group element.
    pub fn transform_by_group_element(
        &self,
        action: &GroupAction<NamedPoint>,
        element: GroupElementId,
    ) -> Self {
        Self(self.0.iter().map(|&p| action.act(element, p)).collect())
    }

    /// Offsets all named point IDs by some amount.
    pub(crate) fn offset_ids_by(&self, id_offset: usize) -> Self {
        Self(
            self.0
                .iter()
                .map(|&p| NamedPoint(p.0 + id_offset as u16))
                .collect(),
        )
    }

    /// Returns the number of named points in the list.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns whether the list is empty.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Returns a vector between all of the named points.
    ///
    /// This vector's stabilizer is presumed to be the setwise stabilizer of the
    /// set of named points.
    pub fn vector(&self, named_point_vectors: &PerNamedPoint<Vector>) -> Vector {
        self.0.iter().map(|&p| &named_point_vectors[p]).sum()
    }

    /// Returns an iterator over the named points in the set.
    pub fn iter(&self) -> <&Self as IntoIterator>::IntoIter {
        (self).into_iter()
    }

    /// Returns the name for the set, in human-friendly notation.
    pub fn name(&self, named_point_names: &NameSpecBiMap<NamedPoint>) -> String {
        self.iter().map(|p| &named_point_names[p]).join("_") // TODO: proper separator
    }
}

impl<'a> IntoIterator for &'a NamedPointSet {
    type Item = NamedPoint;

    type IntoIter = std::iter::Copied<std::slice::Iter<'a, NamedPoint>>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter().copied()
    }
}
