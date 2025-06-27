//! Basic non-mathematical operators `==` and `!=`.

use itertools::Itertools;

use crate::{Error, Result, Scope, Span, Spanned, Value};

const MAX_RANGE_SIZE: usize = 65535;

/// Adds the built-in operators to the scope.
pub fn define_in(scope: &Scope) -> Result<()> {
    scope.register_builtin_functions(hps_fns![
        ("==", |ctx, a: Value, b: Value| -> bool {
            a.eq(&b, ctx.caller_span)?
        }),
        ("!=", |ctx, a: Value, b: Value| -> bool {
            !a.eq(&b, ctx.caller_span)?
        }),
        ("..", |ctx, a: i64, b: i64| -> Vec<Spanned<i64>> {
            try_range(ctx.caller_span, a..b)?
        }),
        ("..=", |ctx, a: i64, b: i64| -> Vec<Spanned<i64>> {
            try_range(ctx.caller_span, a..=b)?
        }),
    ])
}

fn try_range<T>(span: Span, range: impl Iterator<Item = T>) -> Result<Vec<Spanned<T>>> {
    check_iter_len(span, &range)?;
    Ok(range.map(|i| (i, span)).collect())
}

fn check_iter_len<T>(span: Span, iter: &impl Iterator<Item = T>) -> Result<()> {
    if iter.try_len().is_ok_and(|len| len <= MAX_RANGE_SIZE) {
        Ok(())
    } else {
        Err(Error::RangeTooBig {
            len: MAX_RANGE_SIZE,
        }
        .at(span))
    }
}
