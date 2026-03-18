//! Abstract groups defined using lookup tables.
//!
//! These form the basis for all other groups.

mod abstract_group;
mod abstract_group_action;
mod abstract_subgroup;
mod builder;
mod egg;

pub(crate) use abstract_group::AbstractGroupLut;
pub(crate) use abstract_group_action::AbstractGroupActionLut;
pub(crate) use abstract_subgroup::AbstractSubgroup;
pub(crate) use builder::AbstractGroupLutBuilder;
pub(crate) use egg::EggTable;
