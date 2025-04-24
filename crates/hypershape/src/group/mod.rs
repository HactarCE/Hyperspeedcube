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
        .map(|c| match c {
            // Source: https://web.archive.org/web/20230410033043/https://bendwavy.org/klitzing//explain/dynkin-notation.htm
            'o' => Ok(0.0),
            'x' => Ok(1.0),
            'q' => Ok(std::f64::consts::SQRT_2),
            'f' => Ok((5.0_f64.sqrt() + 1.0) * 0.5), // phi
            'u' => Ok(2.0),
            _ => Err(DynkinNotationError::BadChar(c)),
        })
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
