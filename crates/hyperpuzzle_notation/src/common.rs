//! Structures common to [`crate::spanned`] and [`crate::unspanned`].
//!
//! These are also re-exported by each of the above modules.

use std::{fmt, str::FromStr};

/// Kind of group.
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub enum GroupKind {
    /// Simple group.
    ///
    /// Example: `(R U R' U')`
    #[default]
    Simple,
    /// Macro group.
    ///
    /// Example: `!(R U R' U R U2' R')`
    Macro,
    /// Simultaneous group.
    ///
    /// Example: `&(U' D2)`
    Simultaneous,
    /// Normal/inverse scramble switch group.
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
    /// Commutator.
    ///
    /// Example: `[R U R', U]`
    Commutator,
    /// Macro group.
    ///
    /// Example: `[R: U]`
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
