//! Structures common to [`crate::spanned`] and [`crate::unspanned`].
//!
//! These are also re-exported by each of the above modules.

use std::fmt;
use std::str::FromStr;

use crate::InvertError;
pub use crate::layer::{Layer, LayerMask, LayerRange, SignedLayer};

/// Kind of group.
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub enum GroupKind {
    /// Simple group.
    ///
    /// A simple group's move count should be equivalent to the moves on their
    /// own.
    ///
    /// A simple group should affect animation speed slightly or not at all.
    ///
    /// Example: `(R U R' U')`
    #[default]
    Simple,
    /// Macro group.
    ///
    /// A macro group's move count should be equivalent to the moves on their
    /// own.
    ///
    /// A macro group should animate very quickly or not at all.
    ///
    /// Example: `!(R U R' U R U2' R')`
    Macro,
    /// Simultaneous group, which represents moves done simultaneously.
    ///
    /// A simultaneous group should count as 1 move in Execution Turn Metric.
    ///
    /// A simultaneous group should animate simultaneously if possible. It may
    /// animate sequentially, but should take the same amount of time as a
    /// single move.
    ///
    /// Example: `&(U' D2)`
    Simultaneous,
    /// Normal/inverse scramble switch group, which represents moves done on the
    /// inverse puzzle state.
    ///
    /// A NISS group's move count should be equivalent to the moves on their
    /// own.
    ///
    /// A NISS group's moves may be animated inverted before the scramble (most
    /// useful while constructing a solution) or inverted after the forwards
    /// solution (most useful while viewing a completed solution). These two
    /// might not be equivalent if the combined scramble + forwards solution +
    /// NISS solution is not the identity.
    ///
    /// Example: `^(U R' D2)`
    Niss,
}

impl GroupKind {
    /// Returns the prefix symbol for the group, or `None` for a simple group.
    ///
    /// - `!` for macro groups
    /// - `&` for simultaneous groups
    /// - `^` for NISS groups
    pub fn prefix(self) -> Option<char> {
        match self {
            GroupKind::Simple => None,
            GroupKind::Macro => Some('!'),
            GroupKind::Simultaneous => Some('&'),
            GroupKind::Niss => Some('^'),
        }
    }
}

/// Kind of binary group.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub enum BinaryGroupKind {
    /// Commutator `[A, B]` that expands to `A B A' B'`.
    Commutator,
    /// Conjugate `[A: B]` that expands to `A B A'`.
    Conjugate,
}

impl BinaryGroupKind {
    /// Returns the separator symbol for the group. This is `,` for commutators
    /// and `:` for conjugates.
    pub fn separator(self) -> char {
        match self {
            BinaryGroupKind::Commutator => ',',
            BinaryGroupKind::Conjugate => ':',
        }
    }
}

/// Square-1 move.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub enum Sq1Move {
    /// Move of the top and bottom layers.
    ///
    /// Example: `(1, -3)`
    UD {
        /// How far clockwise to move the top layer, as a multiple of 30°.
        u: i32,
        /// How far clockwise to move the bottom layer, as a multiple of 30°.
        d: i32,
    },
    /// Move of the right side by 180°.
    Slash,
}

impl fmt::Display for Sq1Move {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UD { u, d } => write!(f, "({u}, {d})"),
            Self::Slash => write!(f, "/"),
        }
    }
}

impl Sq1Move {
    /// Returns the inverse move.
    pub fn inv(self) -> Result<Self, InvertError> {
        match self {
            Sq1Move::UD { u, d } => Ok(Sq1Move::UD {
                u: u.checked_neg().ok_or(InvertError::IntegerOverflow)?,
                d: d.checked_neg().ok_or(InvertError::IntegerOverflow)?,
            }),
            Sq1Move::Slash => Ok(Sq1Move::Slash),
        }
    }
}

/// WCA Megaminx scrambling move.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub enum MegaminxScrambleMove {
    /// R++
    Rpp,
    /// R--
    Rmm,
    /// D++
    Dpp,
    /// D--
    Dmm,
}

impl fmt::Display for MegaminxScrambleMove {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MegaminxScrambleMove::Rpp => write!(f, "R++"),
            MegaminxScrambleMove::Rmm => write!(f, "R--"),
            MegaminxScrambleMove::Dpp => write!(f, "D++"),
            MegaminxScrambleMove::Dmm => write!(f, "D--"),
        }
    }
}

impl MegaminxScrambleMove {
    /// Returns the inverse move.
    #[must_use]
    pub fn inv(self) -> Self {
        match self {
            MegaminxScrambleMove::Rpp => MegaminxScrambleMove::Rmm,
            MegaminxScrambleMove::Rmm => MegaminxScrambleMove::Rpp,
            MegaminxScrambleMove::Dpp => MegaminxScrambleMove::Dmm,
            MegaminxScrambleMove::Dmm => MegaminxScrambleMove::Dpp,
        }
    }
}

/// Multiplier suffix using `'` for negative numbers.
///
/// The default multiplier is `Multiplier(1)`.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub struct Multiplier(pub i32);

impl Default for Multiplier {
    fn default() -> Self {
        Self(1)
    }
}

impl fmt::Display for Multiplier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // cast to i64 to avoid overflow
        let abs = self.0.abs();
        if abs != 1 {
            write!(f, "{abs}")?;
        }
        if self.0 < 0 {
            write!(f, "'")?;
        }
        Ok(())
    }
}

impl From<i32> for Multiplier {
    fn from(value: i32) -> Self {
        Self(value)
    }
}

impl FromStr for Multiplier {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.strip_suffix('\'') {
            Some(rest) => Ok(Self(
                i32::try_from(rest.parse::<u32>().map_err(|_| ())?)
                    .map_err(|_| ())?
                    .checked_neg()
                    .ok_or(())?,
            )),
            _ => Ok(Self(
                i32::try_from(s.parse::<u32>().map_err(|_| ())?).map_err(|_| ())?,
            )),
        }
    }
}

impl Multiplier {
    /// Returns the inverse multiplier.
    pub fn inv(self) -> Result<Multiplier, InvertError> {
        match self.0.checked_neg() {
            Some(m) => Ok(Self(m)),
            None => Err(InvertError::IntegerOverflow),
        }
    }
}

pub(crate) fn write_separated_list<T: fmt::Display>(
    f: &mut fmt::Formatter<'_>,
    elements: &[T],
    separator: &str,
) -> fmt::Result {
    let mut is_first = true;
    for elem in elements {
        if is_first {
            is_first = false;
        } else {
            write!(f, "{separator}")?;
        }
        write!(f, "{elem}")?;
    }
    Ok(())
}
