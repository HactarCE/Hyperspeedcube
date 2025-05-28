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
        // Base conversion
        hps_fn!("to_digits", |n: Int, base: Int| -> List(Int) {}),
        hps_fn!("from_digits", |digits: List(Int), base: Int| -> Int {}),
        hps_fn!("to_digits_bijective", |n: Int, base: Int| -> List(Int) {}),
        hps_fn!("from_digits_bijective", |digits: List(Int),
                                          base: Int|
         -> Int {}),
        hps_fn!("to_base", |n: Int, base: Int| -> Str {}),
        hps_fn!("to_base", |n: Int, digits: Str| -> Str {}),
        hps_fn!("to_base", |n: Int, digits: List(Str)| -> Str {}),
        hps_fn!("to_base_bijective", |n: Int, base: Int| -> Str {}),
        hps_fn!("to_base_bijective", |n: Int, digits: Str| -> Str {}),
        hps_fn!("to_base_bijective", |n: Int, digits: List(Str)| -> Str {}),
        hps_fn!("from_base", |s: Str, base: Int| -> Int {}),
        hps_fn!("from_base", |s: Str, digits: Str| -> Int {}),
        hps_fn!("from_base", |s: Str, digits: List(Str)| -> Int {}),
        hps_fn!("from_base_bijective", |s: Str, base: Int| -> Int {}),
        hps_fn!("from_base_bijective", |s: Str, digits: Str| -> Int {}),
        hps_fn!("from_base_bijective", |s: Str, digits: List(Str)| -> Int {}),
        hps_fn!("to_a1z26", |n: Int| -> Str {}),
        hps_fn!("from_a1z26", |n: Int| -> Str {}),
    ])
}

fn int_to_char(n: i64, n_span: Span) -> Result<char> {
    n.try_into()
        .ok()
        .and_then(char::from_u32)
        .ok_or(Error::User(eco_format!("invalid codepoint {n}")).at(n_span))
}

fn to_base(span: Span, mut n: i64, digits: &[&str]) -> Result<EcoString> {
    let base = digits.len() as i64;
    if base < 2 {
        return Err(Error::User(eco_format!("expected at least 2 digits; got {base}")).at(span));
    }

    let mut s = EcoString::new();
    if n < 0 {
        s.push('-');
    }
    if n == 0 {
        s += digits[0];
    }
    while n != 0 {
        s += digits[(n % base).unsigned_abs() as usize];
        n /= base;
    }
    Ok(s)
}

fn to_base_bijective(span: Span, mut n: i64, digits: &[&str]) -> Result<EcoString> {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_base() {
        // assert_eq!("A", to_base(crate::BUILTIN_SPAN, 0, "ABCD").unwrap());
        // assert_eq!("B", to_base(crate::BUILTIN_SPAN, 1, "ABCD").unwrap());
        // assert_eq!("C", to_base(crate::BUILTIN_SPAN, 2, "ABCD").unwrap());
        // assert_eq!("D", to_base(crate::BUILTIN_SPAN, 3, "ABCD").unwrap());
        // assert_eq!("BA", to_base(crate::BUILTIN_SPAN, 4, "ABCD").unwrap());
        // assert_eq!("DD", to_base(crate::BUILTIN_SPAN, 15, "ABCD").unwrap());
        // assert_eq!("BAA", to_base(crate::BUILTIN_SPAN, 16, "ABCD").unwrap());
        todo!()
    }
}
