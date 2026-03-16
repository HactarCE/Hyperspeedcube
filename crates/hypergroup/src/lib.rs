//! Data structures and algorithms for finite groups, specifically Coxeter
//! groups.

mod abstract_group;
mod action;
mod common;
mod constraints;
mod coxeter_group;
mod factorization;
mod finite_coxeter_group;
mod isometry_group;
mod product_action;
mod product_constraints;
mod product_group;
mod product_subgroup;
mod subgroup;

pub use abstract_group::{AbstractGroup, Group, GroupBuilder};
use action::SubgroupOrbits;
pub use action::{GroupAction, PerRefPoint, RefPoint};
pub use common::*;
pub use constraints::{Constraint, ConstraintSet, ConstraintSolver};
pub use coxeter_group::*;
pub use factorization::{Factorization, FactorizationIntoIter};
pub use finite_coxeter_group::FiniteCoxeterGroup;
pub use isometry_group::IsometryGroup;
pub use product_action::ProductGroupAction;
pub use product_constraints::ProductConstraintSolver;
pub use product_group::ProductGroup;
use product_group::{FactorGroup, PerFactorGroup};
pub use product_subgroup::ProductSubgroup;
pub use subgroup::{ConjugateCoset, Subgroup};

/// Parses a single character of a vector in limited Dynkin notation, where `o`
/// represents `0` and `x` represents `1`. Returns `None` for all other
/// characters.
///
/// Source: <https://bendwavy.org/klitzing/explain/dynkin-notation.htm>
pub fn dynkin_char(c: char) -> Option<hypermath::Float> {
    match c {
        'o' => Some(0.0),
        'x' => Some(1.0),
        // Other characters exist, but we don't have a use for them yet.
        // 'q' => Some(std::f64::consts::SQRT_2),
        // 'f' => Some((5.0_f64.sqrt() + 1.0) * 0.5), // phi
        // 'u' => Some(2.0),
        _ => None,
    }
}

/// Parses a vector in Dynkin notation. For example, `oox` represents `[0, 0,
/// 1]`.
pub fn parse_dynkin_notation(
    ndim: u8,
    s: &str,
) -> Result<hypermath::Vector, DynkinNotationError<'_>> {
    if s.len() != ndim as usize {
        return Err(DynkinNotationError::BadLength {
            ndim,
            len: s.len(),
            s,
        });
    }
    s.chars()
        .map(|c| dynkin_char(c).ok_or(DynkinNotationError::BadChar(c)))
        .collect()
}

/// Error emitted by [`parse_dynkin_notation()`].
#[expect(missing_docs)]
#[derive(thiserror::Error, Debug, Clone)]
pub enum DynkinNotationError<'a> {
    #[error("group has ndim {ndim} but string {s:?} has length {len}")]
    BadLength { ndim: u8, len: usize, s: &'a str },
    #[error("invalid character {0:?} for coxeter vector. supported characters: [o, x, q, f, u]")]
    BadChar(char),
}
