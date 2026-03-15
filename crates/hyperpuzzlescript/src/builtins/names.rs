//! Base conversion functions.

use ecow::eco_format;
use hypuz_notation::family;

use crate::{Builtins, ListOf, Result, Str};

/// Adds the built-in functions.
pub fn define_in(builtins: &mut Builtins<'_>) -> Result<()> {
    builtins.set_fns(hps_fns![
        /// Returns the standard prefix for the `n`th opposite axis, starting at
        /// `β` = 1. If `n` is omitted, it is assumed to be `1`.
        ///
        /// - 0 uses the empty string
        /// - 1 uses `β`
        /// - 2 uses `δ`
        /// - etc.
        ///
        /// It is very rare for anything other than `opposite(1)` to be
        /// necessary.
        fn opposite() -> Str {
            eco_format!("{}", family::Opposite(1))
        }
        fn opposite_axis_prefix(n: u32) -> Str {
            eco_format!("{}", family::Opposite(n))
        }

        /// Returns the standard `n`th sequential lowercase name, starting at
        /// `a` = 0. This is typically used for numbering sets of axes.
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
        fn nth_lowercase(n: u32) -> Str {
            eco_format!("{}", family::SequentialLowercaseName(n))
        }

        /// Returns the standard `n`th sequential uppercase name, starting at
        /// `A` = 0. This is typically used for numbering axes within a set.
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
        fn nth_uppercase(n: u32) -> Str {
            eco_format!("{}", family::SequentialUppercaseName(n))
        }

        /// Parses a standard name into 3 numbers:
        ///
        /// - Opposite index
        /// - Lowercase prefix (may be `null`)
        /// - Uppercase index
        fn parse_name((s, span): Str) -> Option<ListOf<Option<u32>>> {
            // IIFE to mimic try_block
            let (family::Opposite(opp), s) = family::strip_opposite_axis_prefix(&s);
            let (low, s) = match family::strip_sequential_lowercase_prefix(s) {
                Some((family::SequentialLowercaseName(low), rest)) => (Some(low), rest),
                None => (None, s),
            };
            s.parse().ok().map(|family::SequentialUppercaseName(upp)| {
                vec![(Some(opp), span), (low, span), (Some(upp), span)]
            })
        }

        /// Parses a standard opposite axis prefix and returns its index, or
        /// `null` if it is invalid.
        fn parse_opposite_prefix(s: Str) -> Option<u32> {
            s.parse().ok().map(|family::Opposite(n)| n)
        }

        /// Parses a standard sequential lowercase name and returns its index,
        /// or `null` if it is invalid.
        fn parse_nth_lowercase(s: Str) -> Option<u32> {
            s.parse().ok().map(|family::SequentialLowercaseName(n)| n)
        }

        /// Parses a standard sequential uppercase name and returns its index,
        /// or `null` if it is invalid.
        fn parse_nth_uppercase(s: Str) -> Option<u32> {
            s.parse().ok().map(|family::SequentialUppercaseName(n)| n)
        }
    ])
}
