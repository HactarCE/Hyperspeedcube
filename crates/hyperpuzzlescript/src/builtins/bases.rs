use ecow::{EcoString, eco_format};
use itertools::Itertools;

use crate::{Error, Result, Scope, Span};

const A0Z25: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZ";
const DEFAULT_BASE_DIGITS: &str = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ";

pub fn define_in(scope: &Scope) -> Result<()> {
    scope.register_builtin_functions([
        // Digits
        hps_fn!("to_digits", |ctx,
                              n: Nat,
                              (base, base_span): Nat|
         -> List(Nat) {
            to_digits(n, base, base_span, false)?
                .into_iter()
                .map(|digit| (digit as f64, ctx.caller_span))
                .collect_vec()
        }),
        hps_fn!("from_digits", |(digits, digits_span): List(Nat),
                                (base, base_span): Nat|
         -> Nat {
            from_digits(&digits, digits_span, base, base_span, false)?
        }),
        hps_fn!("to_digits_bijective", |ctx,
                                        n: Nat,
                                        (base, base_span): Nat|
         -> List(Nat) {
            to_digits(n, base, base_span, true)?
                .into_iter()
                .map(|digit| (digit as f64, ctx.caller_span))
                .collect_vec()
        }),
        hps_fn!(
            "from_digits_bijective",
            |(digits, digits_span): List(Nat), (base, base_span): Nat| -> Nat {
                from_digits(&digits, digits_span, base, base_span, true)?
            }
        ),
        // to_base
        hps_fn!("to_base", |(n, n_span): Nat,
                            (base, base_span): Nat|
         -> Str {
            let base_digits = default_base_digits(base, base_span)?;
            let digits = to_digits(n, base, base_span, false)?;
            digits_to_str(&digits, n_span, base_digits)?
        }),
        hps_fn!("to_base", |(n, n_span): Nat,
                            (base_digits, base_span): Str|
         -> Str {
            let base = base_digits.chars().count() as u64;
            let digits = to_digits(n, base, base_span, false)?;
            digits_to_str(&digits, n_span, &base_digits)?
        }),
        // to_base_bijective
        hps_fn!("to_base_bijective", |(n, n_span): Nat,
                                      (base, base_span): Nat|
         -> Str {
            let base_digits = default_base_digits(base, base_span)?;
            let digits = to_digits(n, base, base_span, true)?;
            digits_to_str(&digits, n_span, base_digits)?
        }),
        hps_fn!("to_base_bijective", |(n, n_span): Nat,
                                      (base_digits, base_span): Str|
         -> Str {
            let base = base_digits.chars().count() as u64;
            let digits = to_digits(n, base, base_span, true)?;
            digits_to_str(&digits, n_span, &base_digits)?
        }),
        // from_base
        hps_fn!("from_base", |(s, s_span): Str,
                              (base, base_span): Nat|
         -> Nat {
            let base_digits = default_base_digits(base, base_span)?;
            let digits = str_to_digits(&s, s_span, base_digits)?;
            from_digits(&digits, s_span, base, base_span, false)?
        }),
        hps_fn!("from_base", |(s, s_span): Str,
                              (base_digits, base_span): Str|
         -> Nat {
            let base = base_digits.chars().count() as u64;
            let digits = str_to_digits(&s, s_span, &base_digits)?;
            from_digits(&digits, s_span, base, base_span, false)?
        }),
        // from_base_bijective
        hps_fn!("from_base_bijective", |(s, s_span): Str,
                                        (base, base_span): Nat|
         -> Nat {
            let base_digits = default_base_digits(base, base_span)?;
            let digits = str_to_digits(&s, s_span, base_digits)?;
            from_digits(&digits, s_span, base, base_span, true)?
        }),
        hps_fn!("from_base_bijective", |(s, s_span): Str,
                                        (base_digits, base_span): Str|
         -> Nat {
            let base = base_digits.chars().count() as u64;
            let digits = str_to_digits(&s, s_span, &base_digits)?;
            from_digits(&digits, s_span, base, base_span, true)?
        }),
        // base26
        hps_fn!("to_base26", |(n, n_span): Nat| -> Str {
            let digits = to_digits(n, 26, crate::BUILTIN_SPAN, true)?;
            digits_to_str(&digits, n_span, A0Z25)?
        }),
        hps_fn!("from_base26", |(s, s_span): Str| -> Nat {
            let digits = str_to_digits(&s, s_span, A0Z25)?;
            from_digits(&digits, s_span, 26, crate::BUILTIN_SPAN, true)?
        }),
    ])
}

fn default_base_digits(base: u64, base_span: Span) -> Result<&'static str> {
    DEFAULT_BASE_DIGITS.get(..base as usize).ok_or_else(|| {
        Error::User(eco_format!(
            "base {base} is too big; you must specify digits"
        ))
        .at(base_span)
    })
}

fn str_to_digits(s: &str, s_span: Span, base_digits: &str) -> Result<Vec<u64>> {
    s.chars()
        .map(|c| {
            base_digits
                .find(c)
                .ok_or_else(|| Error::User(eco_format!("unknown digit {c:?}")).at(s_span))
                .map(|i| i as u64)
        })
        .collect()
}
fn digits_to_str(digits: &[u64], digits_span: Span, base_digits: &str) -> Result<EcoString> {
    let chars = base_digits.chars().collect_vec();
    let base = chars.len();
    digits
        .iter()
        .map(|i| {
            chars.get(*i as usize).copied().ok_or_else(|| {
                Error::User(eco_format!("digit {i} is invalid in base {base}")).at(digits_span)
            })
        })
        .collect()
}

fn check_base(base: u64, min_base: u64, base_span: Span) -> Result<()> {
    if base < min_base {
        Err(Error::User(eco_format!("cannot convert number to base {base}")).at(base_span))
    } else {
        Ok(())
    }
}
fn to_digits(mut n: u64, base: u64, base_span: Span, bijective: bool) -> Result<Vec<u64>> {
    check_base(base, 2, base_span)?;
    if n == 0 {
        Ok(vec![0])
    } else {
        let mut ret = vec![];
        loop {
            ret.push(n % base);
            n /= base;
            if n == 0 {
                break;
            } else if bijective {
                n -= 1;
            }
        }
        ret.reverse();
        Ok(ret)
    }
}
fn from_digits(
    digits: &[u64],
    digits_span: Span,
    base: u64,
    base_span: Span,
    bijective: bool,
) -> Result<f64> {
    check_base(base, 2, base_span)?;
    if digits.is_empty() {
        return Err(Error::User("cannot convert empty string to number".into()).at(digits_span));
    }
    let mut unit = 1.0;
    let mut ret = if bijective { -1.0 } else { 0.0 };
    for &digit in digits.iter().rev() {
        if digit >= base {
            return Err(
                Error::User(eco_format!("digit {digit} is too big for base {base}"))
                    .at(digits_span),
            );
        }
        ret += (digit as f64 + bijective as u8 as f64) * unit;
        unit *= base as f64;
    }
    Ok(ret)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_base() {
        let span = crate::BUILTIN_SPAN;

        let base = 3;
        let test_cases: &[&[u64]] = &[
            &[0],
            &[1],
            &[2],
            &[1, 0],
            &[1, 1],
            &[1, 2],
            &[2, 0],
            &[2, 1],
            &[2, 2],
            &[1, 0, 0],
            &[1, 0, 1],
        ];
        for (i, &test_case) in test_cases.iter().enumerate() {
            assert_eq!(test_case, to_digits(i as u64, base, span, false).unwrap());
            assert_eq!(
                i as f64,
                from_digits(test_case, span, base, span, false).unwrap()
            );
        }

        let base = 3;
        let test_cases: &[&[u64]] = &[
            &[0],
            &[1],
            &[2],
            &[0, 0],
            &[0, 1],
            &[0, 2],
            &[1, 0],
            &[1, 1],
            &[1, 2],
            &[2, 0],
            &[2, 1],
            &[2, 2],
            &[0, 0, 0],
            &[0, 0, 1],
        ];
        for (i, &test_case) in test_cases.iter().enumerate() {
            assert_eq!(test_case, to_digits(i as u64, base, span, true).unwrap());
            assert_eq!(
                i as f64,
                from_digits(test_case, span, base, span, true).unwrap()
            );
        }
    }
}
