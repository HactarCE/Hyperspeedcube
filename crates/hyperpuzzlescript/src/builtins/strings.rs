//! Operators and functions for operating on lists and maps.

use ecow::{EcoString, eco_format};
use itertools::Itertools;

use crate::{Builtins, Error, ListOf, Num, Result, Span, Str, ValueData};

/// Adds the built-in operators and functions.
pub fn define_in(builtins: &mut Builtins<'_>) -> Result<()> {
    builtins.set_fns(hps_fns![
        // Concatenation
        ("++", |_, s1: Str, s2: Str| -> Str { s1 + s2 }),
        ("join", |_, strings: Vec<Str>| -> String {
            strings.into_iter().join("")
        }),
        ("join", |_, strings: Vec<Str>, sep: Str| -> String {
            strings.into_iter().join(&sep)
        }),
        // String conversion
        ("str", |_, arg: ValueData| -> Str { eco_format!("{arg}") }),
        ("repr", |_, arg: ValueData| -> Str {
            eco_format!("{arg:?}")
        }),
        // Case conversion
        ("upper", |_, s: Str| -> Str { s.to_uppercase() }),
        ("lower", |_, s: Str| -> Str { s.to_lowercase() }),
        ("snake", |_, s: Str| -> Str { s.replace(" ", "_") }),
        ("capital", |_, s: Str| -> Str {
            let first_char_len = s.chars().next().map(char::len_utf8).unwrap_or(0);
            (s[..first_char_len].to_uppercase() + &s[first_char_len..]).into()
        }),
        // Unicode
        ("chars", |ctx, s: Str| -> ListOf<Str> {
            s.chars().map(|c| (c.into(), ctx.caller_span)).collect_vec()
        }),
        ("ord", |_, (s, s_span): Str| -> Num {
            match s.chars().exactly_one() {
                Ok(c) => c as u32 as Num,
                Err(_) => {
                    return Err(Error::BadArgument {
                        value: ValueData::Str(s).repr(),
                        note: Some(
                            "string must have exactly one \
                             character (Unicode codepoint)"
                                .to_owned(),
                        ),
                    }
                    .at(s_span));
                }
            }
        }),
        ("chr", |_, (n, n_span): i64| -> Str {
            eco_format!("{}", int_to_char(n, n_span)?)
        }),
        // Splitting
        ("split", |ctx, s: Str, pattern: Str| -> ListOf<&str> {
            s.split(&*pattern)
                .map(|sub| (sub, ctx.caller_span))
                .collect_vec()
        }),
        // Other operations
        ("rev", |_, s: Str| -> Str {
            s.chars().rev().collect::<EcoString>()
        }),
    ])
}

fn int_to_char(n: i64, n_span: Span) -> Result<char> {
    n.try_into()
        .ok()
        .and_then(char::from_u32)
        .ok_or(Error::User(eco_format!("invalid codepoint {n}")).at(n_span))
}
