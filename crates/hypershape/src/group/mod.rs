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

/// Parses a single character of a vector in limited Dynkin notation, where `o`
/// represents `0` and `x` represents `1`. Returns `None` for all other
/// characters.
///
/// Source: https://bendwavy.org/klitzing/explain/dynkin-notation.htm
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
#[allow(missing_docs)]
#[derive(thiserror::Error, Debug, Clone)]
pub enum DynkinNotationError<'a> {
    #[error("group has ndim {ndim} but string {s:?} has length {len}")]
    BadLength { ndim: u8, len: usize, s: &'a str },
    #[error("invalid character {0:?} for coxeter vector. supported characters: [o, x, q, f, u]")]
    BadChar(char),
}
