//! Data structures and algorithms for finite groups, specifically Coxeter
//! groups.

mod action;
mod constraints;
mod coset;
mod coxeter;
mod errors;
mod gen_seq;
mod geometry;
mod group;
mod orbit_helpers;
mod primitives;
mod subgroup_action;

pub use action::GroupAction;
pub use constraints::{
    ConjugateSubgroupConstraintSolver, Constraint, ConstraintSet, ConstraintSolver,
    SubgroupConstraintSolver,
};
pub use coset::{ConjugateCoset, LeftCoset, RightCoset, Subgroup};
pub use coxeter::{CoxeterMatrix, DynkinNotationError, dynkin_char, parse_dynkin_notation};
pub use errors::{GroupError, GroupResult};
pub use gen_seq::*;
use geometry::FactorGroupIsometries;
pub use geometry::IsometryGroup;
pub use group::Group;
pub use orbit_helpers::*;
use primitives::{
    AbstractGroupActionLut, AbstractGroupLut, AbstractGroupLutBuilder, AbstractSubgroup, EggTable,
};
pub use subgroup_action::SubgroupAction;

/// Recommended limit for group construction.
pub const ORBIT_LIMIT: usize = 1_000_000;

hypuz_util::typed_index_struct! {
    /// ID of a group generator.
    ///
    /// These have no correlation with group element IDs.
    pub struct GeneratorId(pub u8);
    /// ID of a group element.
    ///
    /// `GroupElementId(0)` is always the [identity element].
    ///
    /// [identity element]: https://en.wikipedia.org/wiki/Identity_element
    pub struct GroupElementId(pub u32);

    /// Factor group that makes up a [`crate::Group`].
    pub(crate) struct FactorGroup(u8);
}

impl GroupElementId {
    /// Identity element in any group.
    pub const IDENTITY: GroupElementId = GroupElementId(0);
}

/// List containing a value per group generator.
pub type PerGenerator<T> = hypuz_util::ti::TiVec<GeneratorId, T>;
/// List containing a value per group element.
pub type PerGroupElement<T> = hypuz_util::ti::TiVec<GroupElementId, T>;

/// List containing a value per factor group.
pub(crate) type PerFactorGroup<T> = hypuz_util::ti::TiVec<FactorGroup, T>;

#[cfg(test)]
mod tests;
