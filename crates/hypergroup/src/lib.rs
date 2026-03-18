//! Data structures and algorithms for finite groups, specifically Coxeter
//! groups.

mod action;
mod common;
mod constraints;
mod coset;
mod coxeter;
mod errors;
mod gen_seq;
mod geometry;
mod group;
mod primitives;

use geometry::FactorGroupIsometries;
use primitives::{
    AbstractGroupActionLut, AbstractGroupLut, AbstractGroupLutBuilder, AbstractSubgroup, EggTable,
};

pub use action::GroupAction;
pub use common::*;
pub use constraints::{Constraint, ConstraintSet, ConstraintSolver};
pub use coset::Coset;
pub use coxeter::{
    Coxeter, CoxeterMatrix, DynkinNotationError, dynkin_char, parse_dynkin_notation,
};
pub use errors::{GroupError, GroupResult};
pub use gen_seq::*;
pub use geometry::IsometryGroup;
pub use group::Group;

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
