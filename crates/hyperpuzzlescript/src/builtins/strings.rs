use ecow::{EcoString, eco_format};
use itertools::Itertools;

use crate::{Error, Result, Scope, Span, ValueData};

pub fn define_in(scope: &Scope) -> Result<()> {
    scope.register_builtin_functions([
        // Concatenation
        hps_fn!("++", |s1: Str, s2: Str| -> Str { s1 + s2 }),
        hps_fn!("join", |strings: List(Str)| -> Str {
            strings.into_iter().join("")
        }),
        hps_fn!("join", |strings: List(Str), sep: Str| -> Str {
            strings.into_iter().join(&sep)
        }),
        // String conversion
        hps_fn!("str", |arg: Any| -> Str { eco_format!("{arg}") }),
        hps_fn!("repr", |arg: Any| -> Str { eco_format!("{:?}", arg.data) }),
        // Case conversion
        hps_fn!("upper", |s: Str| -> Str { s.to_uppercase() }),
        hps_fn!("lower", |s: Str| -> Str { s.to_lowercase() }),
        // Unicode
        hps_fn!("chars", |ctx, s: Str| -> List(Str) {
            s.chars()
                .map(|c| ValueData::Str(c.into()).at(ctx.caller_span))
                .collect_vec()
        }),
        hps_fn!("ord", |(s, s_span): Str| -> Int {
            match s.chars().exactly_one() {
                Ok(c) => c as u32 as f64,
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
        hps_fn!("chr", |(n, n_span): Int| -> Str {
            eco_format!("{}", int_to_char(n, n_span)?)
        }),
        // Splitting
        hps_fn!("split", |ctx, s: Str, pattern: Str| -> List(Str) {
            s.split(&*pattern)
                .map(|sub| ValueData::Str(sub.into()).at(ctx.caller_span))
                .collect_vec()
        }),
        // Other operations
        hps_fn!("rev", |s: Str| -> Str {
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
