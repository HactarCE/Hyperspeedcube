//! Utility functions for working with common family names for moves and
//! rotations.

use std::{fmt, ops::RangeInclusive, str::FromStr};

use chumsky::container::Seq;
use smallvec::SmallVec;

pub use crate::charsets::{LARGE_LOWERCASE_GREEK, SMALL_LOWERCASE_GREEK, UPPERCASE_GREEK};

/// Removes a jumbling suffix `h`, `j`, or `k` from a twist family.
///
/// The first element of the returned tuple is the twist name without the
/// jumbling suffix. The second element is the jumbling suffix, if one is
/// present.
///
/// If no jumbling suffix is present, the first element of the tuple is the same
/// as the input string.
pub fn strip_jumbling_suffix(s: &str) -> (&str, Option<char>) {
    match s.chars().next_back() {
        Some(c) if is_jumbling_suffix(c) => (&s[..s.len() - c.len_utf8()], Some(c)),
        _ => (s, None),
    }
}

fn is_jumbling_suffix(c: char) -> bool {
    matches!(c, 'h' | 'j' | 'k')
}

/// Removes a sequential lowercase name from the beginnning of `s` if one is
/// present, returning the prefix and the remainder of `s`. Returns `None` if
/// it could not be parsed.
pub fn strip_sequential_lowercase_prefix(s: &str) -> Option<(SequentialLowercaseName, &str)> {
    // +1 is ok because Latin letters are exactly 1 byte
    let i = s.find(|c| matches!(c, 'a'..='z'))? + 1;
    Some((s[..i].parse().ok()?, &s[i..]))
}

/// Removes an opposite axis prefix from the beginning of `s`. This always
/// suceeds, because an opposite axis prefix may be empty.
///
/// Returns `Opposite(0)` in case of overflow.
pub fn strip_opposite_axis_prefix(s: &str) -> (Opposite, &str) {
    let i = s
        .find(|c| !LARGE_LOWERCASE_GREEK.contains(&c))
        .unwrap_or(s.len());
    match s[..i].parse() {
        Ok(opp) => (opp, &s[i..]),
        Err(_) => (Opposite(0), s),
    }
}

/// Index of an opposite axis.
///
/// This is `0` for the original axis (represented using an empty string) and
/// `1` for the opposite axis (represented using `β`). Puzzles with additional
/// opposites, such as the Klein Quartic, should use higher numbers.
pub struct Opposite(pub u32);

impl fmt::Display for Opposite {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for digit in bijective_base_digits(&LARGE_LOWERCASE_GREEK, self.0) {
            write!(f, "{digit}")?;
        }
        Ok(())
    }
}

impl FromStr for Opposite {
    type Err = ParseSequentialNameError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        parse_bijective_base(&LARGE_LOWERCASE_GREEK, s).map(Self)
    }
}

/// Zero-indexed sequential lowercase name.
///
/// This is typically used for numbering sets of axes.
///
/// - 0 = `a`
/// - 1 = `b`
/// - ...
/// - 25 = `z`
/// - 26 = `Γa`
/// - 27 = `Γb`
/// - ...
/// - 51 = `Γz`
/// - 52 = `Δa`
/// - ...
/// - 285 = `Ωz`
/// - 286 = `ΓΓa`
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, Hash)]
pub struct SequentialLowercaseName(pub u32);

impl SequentialLowercaseName {
    /// Returns the uppercase Greek prefix for the name.
    pub fn greek_prefix(self) -> UppercaseGreekPrefix {
        UppercaseGreekPrefix(self.0 / 26)
    }

    /// Returns the Latin letter for the name.
    pub fn latin_letter(self) -> char {
        (b'a' + (self.0 % 26) as u8) as char
    }
}

impl fmt::Display for SequentialLowercaseName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}", self.greek_prefix(), self.latin_letter())
    }
}

impl FromStr for SequentialLowercaseName {
    type Err = ParseSequentialNameError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        parse_sequential_name(b'a'..=b'z', s).map(Self)
    }
}

/// Zero-indexed sequential uppercase name.
///
/// This is typically used for numbering axes within a set.
///
/// - 0 = `A`
/// - 1 = `B`
/// - ...
/// - 25 = `Z`
/// - 26 = `ΓA`
/// - 27 = `ΓB`
/// - ...
/// - 51 = `ΓZ`
/// - 52 = `ΔA`
/// - ...
/// - 285 = `ΩZ`
/// - 286 = `ΓΓA`
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, Hash)]
pub struct SequentialUppercaseName(pub u32);

impl SequentialUppercaseName {
    /// Returns the uppercase Greek prefix for the name.
    pub fn greek_prefix(self) -> UppercaseGreekPrefix {
        UppercaseGreekPrefix(self.0 / 26)
    }

    /// Returns the Latin letter for the name.
    pub fn latin_letter(self) -> char {
        (b'A' + (self.0 % 26) as u8) as char
    }
}

impl fmt::Display for SequentialUppercaseName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}", self.greek_prefix(), self.latin_letter())
    }
}

impl FromStr for SequentialUppercaseName {
    type Err = ParseSequentialNameError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        parse_sequential_name(b'A'..=b'Z', s).map(Self)
    }
}

fn parse_sequential_name(
    latin_alphabet: RangeInclusive<u8>,
    s: &str,
) -> Result<u32, ParseSequentialNameError> {
    let last_byte = s.as_bytes().last().ok_or(ParseSequentialNameError)?;
    if !latin_alphabet.contains(last_byte) {
        return Err(ParseSequentialNameError);
    }
    // last byte is ASCII, so it's safe to slice here
    let greek_prefix = s[..s.len() - 1].parse::<UppercaseGreekPrefix>()?;
    Ok(greek_prefix
        .0
        .checked_mul(latin_alphabet.len() as u32)
        .ok_or(ParseSequentialNameError)?
        .checked_add((*last_byte - latin_alphabet.start()) as u32)
        .ok_or(ParseSequentialNameError)?)
}

/// One-indexed sequential uppercase Greek prefix.
///
/// - 0 = empty string
/// - 1 = `Γ`
/// - ...
/// - 10 = `Ω`
/// - 11 = `ΓΓ`
/// - 20 = `ΓΩ`
/// - 21 = `ΔΓ`
/// - ...
/// - 110 = `ΩΩ`
/// - 111 = `ΓΓΓ`
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, Hash)]
pub struct UppercaseGreekPrefix(pub u32);

impl fmt::Display for UppercaseGreekPrefix {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for digit in bijective_base_digits(&UPPERCASE_GREEK, self.0) {
            write!(f, "{digit}")?;
        }
        Ok(())
    }
}

impl FromStr for UppercaseGreekPrefix {
    type Err = ParseSequentialNameError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        parse_bijective_base(&UPPERCASE_GREEK, s).map(Self)
    }
}

/// Returns the `n`th letter from the small lowercase Greek alphabet series.
///
/// Panics if `n >= 9`.
pub fn nth_small_lowercase_greek(n: usize) -> char {
    SMALL_LOWERCASE_GREEK[n]
}

/// Returns the digits (from most-significant to least-significant)
fn bijective_base_digits(digits: &[char], n: u32) -> impl Iterator<Item = char> {
    let base = digits.len() as u32;
    std::iter::successors(n.checked_sub(1), |&i| (i / base).checked_sub(1))
        .map(|i| (i % base) as u8)
        .collect::<SmallVec<[u8; 24]>>()
        .into_iter()
        .rev()
        .map(|i| digits[i as usize])
}

fn parse_bijective_base(digits: &[char], s: &str) -> Result<u32, ParseSequentialNameError> {
    let mut ret = 0_usize;
    for digit in s.chars() {
        // IIFE to mimic try_block
        let i = digits
            .iter()
            .position(|&c| c == digit)
            .ok_or(ParseSequentialNameError)?;
        ret = (|| {
            ret.checked_mul(digits.len())?
                .checked_add(1)?
                .checked_add(i)
        })()
        .ok_or(ParseSequentialNameError)?;
    }
    ret.try_into().map_err(|_| ParseSequentialNameError)
}

/// Error when parsing a sequential name.
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, Hash)]
pub struct ParseSequentialNameError;

impl fmt::Display for ParseSequentialNameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid sequential name")
    }
}

impl std::error::Error for ParseSequentialNameError {}
