//! Data structures and algorithms for finite groups, specifically Coxeter
//! groups.

mod abstract_group;
mod common;
mod coxeter_group;
mod finite_coxeter_group;
mod isometry_group;

pub use abstract_group::{AbstractGroup, Group, GroupBuilder};
pub use common::*;
pub use coxeter_group::*;
pub use finite_coxeter_group::FiniteCoxeterGroup;
pub use isometry_group::IsometryGroup;
