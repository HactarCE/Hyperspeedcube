//! Utility functions for working with common bracketed transform syntaxes.

use std::fmt;
use std::str::FromStr;

use crate::{Multiplier, Str, write_separated_list};

/// Simultaneous transforms separated by `|`.
///
/// Example: `[R->F | U->I]`
pub struct BracketedSimultaneousTransforms(pub Vec<Str>);

impl fmt::Display for BracketedSimultaneousTransforms {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write_separated_list(f, &self.0, " | ")
    }
}

impl FromStr for BracketedSimultaneousTransforms {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(s.split('|').map(|t| t.trim().into()).collect()))
    }
}

/// Sequential transforms separated by whitespace.
///
/// Example: `[1 j']`
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
pub struct BracketedSequentialTransforms(pub Vec<Str>);

impl fmt::Display for BracketedSequentialTransforms {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write_separated_list(f, &self.0, " ")
    }
}

/// Single transform in a list of sequential transforms.
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
pub struct BracketedTransform {
    /// Transform name
    pub name: Str,
    /// Multiplier.
    ///
    /// If there is not multiplier, then this is `1`.
    pub multiplier: Multiplier,
}

impl fmt::Display for BracketedTransform {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self { name, multiplier } = self;
        if name.is_empty() && *multiplier == Multiplier(1) {
            write!(f, "1")
        } else {
            write!(f, "{name}{multiplier}")
        }
    }
}

impl FromStr for BracketedTransform {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut i = s.len();

        while matches!(s.chars().last(), Some('\'' | '0'..='9')) {
            i -= 1; // ASCII
        }

        Ok(Self {
            name: s[..i].into(),
            multiplier: s[i..].parse()?,
        })
    }
}
