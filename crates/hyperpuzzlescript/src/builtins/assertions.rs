//! Assertion functions `assert()`, `assert_eq()`, `assert_neq()`, and `__eval_to_error()`.

use std::sync::Arc;

use ecow::{EcoString, eco_format};

use crate::{Builtins, Diagnostic, Error, FnValue, Map, Result, Span, Value, ValueData};

/// Adds the built-in functions.
pub fn define_in(builtins: &mut Builtins<'_>) -> Result<()> {
    builtins.set_fns(hps_fns![
        ("assert", |_, (cond, span): bool| -> () {
            assert(cond, || "assertion failed", span)?;
        }),
        ("assert", |_, (cond, span): bool, msg: ValueData| -> () {
            assert(cond, || eco_format!("assertion failed: {msg}"), span)?;
        }),
        ("assert_eq", |ctx, a: Value, b: Value| -> () {
            assert_cmp(
                a.eq(&b, ctx.caller_span)?,
                (a, b),
                || eco_format!("assertion failed"),
                ctx.caller_span,
            )?;
        }),
        (
            "assert_eq",
            |ctx, a: Value, b: Value, msg: ValueData| -> () {
                assert_cmp(
                    a.eq(&b, ctx.caller_span)?,
                    (a, b),
                    || eco_format!("assertion failed: {msg}"),
                    ctx.caller_span,
                )?;
            }
        ),
        ("assert_neq", |ctx, a: Value, b: Value| -> () {
            assert_cmp(
                !a.eq(&b, ctx.caller_span)?,
                (a, b),
                || eco_format!("assertion failed"),
                ctx.caller_span,
            )?;
        }),
        (
            "assert_neq",
            |ctx, a: Value, b: Value, msg: ValueData| -> () {
                assert_cmp(
                    !a.eq(&b, ctx.caller_span)?,
                    (a, b),
                    || eco_format!("assertion failed: {msg}"),
                    ctx.caller_span,
                )?;
            }
        ),
    ])?;

    if super::INCLUDE_DEBUG_FNS {
        builtins.set_fns(hps_fns![(
            "__eval_to_error",
            |ctx, f: Arc<FnValue>| -> String {
                let args = vec![];
                let kwargs = Map::default();
                match f.call_at(ctx.caller_span, ctx.caller_span, ctx, args, kwargs) {
                    Ok(value) => Err(Error::User(eco_format!(
                        "expected error; got {}",
                        value.repr()
                    ))
                    .at(ctx.caller_span)),
                    Err(e) => Ok(match e.msg {
                        Diagnostic::Error(e) => e.to_string(),
                        Diagnostic::Warning(w) => w.to_string(),
                    }),
                }?
            }
        )])?;
    }

    Ok(())
}

fn assert<S: Into<EcoString>>(condition: bool, msg: impl FnOnce() -> S, span: Span) -> Result<()> {
    match condition {
        true => Ok(()),
        false => Err(Error::Assert(msg().into()).at(span)),
    }
}

fn assert_cmp<S: Into<EcoString>>(
    condition: bool,
    (l, r): (Value, Value),
    msg: impl FnOnce() -> S,
    span: Span,
) -> Result<()> {
    match condition {
        true => Ok(()),
        false => Err(Error::AssertCompare(Box::new(l), Box::new(r), msg().into()).at(span)),
    }
}
