use crate::{Diagnostic, Error, Result, Scope, Span, Value};
use ecow::{EcoString, eco_format};

pub fn define_in(scope: &Scope) -> Result<()> {
    scope.register_builtin_functions([
        hps_fn!("assert", |ctx, cond: Bool| -> Null {
            assert(cond, || "assertion failed", ctx.caller_span)?
        }),
        hps_fn!("assert", |ctx, cond: Bool, msg: Any| -> Null {
            assert(
                cond,
                || eco_format!("assertion failed: {msg}"),
                ctx.caller_span,
            )?
        }),
        hps_fn!("assert_eq", |ctx, a: Any, b: Any| -> Null {
            assert_cmp(
                a.eq(&b, ctx.caller_span)?,
                (a, b),
                || eco_format!("assertion failed"),
                ctx.caller_span,
            )?
        }),
        hps_fn!("assert_eq", |ctx, a: Any, b: Any, msg: Any| -> Null {
            assert_cmp(
                a.eq(&b, ctx.caller_span)?,
                (a, b),
                || eco_format!("assertion failed: {msg}"),
                ctx.caller_span,
            )?
        }),
        hps_fn!("assert_neq", |ctx, a: Any, b: Any| -> Null {
            assert_cmp(
                !a.eq(&b, ctx.caller_span)?,
                (a, b),
                || eco_format!("assertion failed"),
                ctx.caller_span,
            )?
        }),
        hps_fn!("assert_neq", |ctx, a: Any, b: Any, msg: Any| -> Null {
            assert_cmp(
                !a.eq(&b, ctx.caller_span)?,
                (a, b),
                || eco_format!("assertion failed: {msg}"),
                ctx.caller_span,
            )?
        }),
        hps_fn!("__eval_to_error", |ctx, f: Fn| -> Str {
            match f.call(ctx.caller_span, ctx.caller_span, ctx, vec![]) {
                Ok(value) => Err(
                    Error::User(eco_format!("expected error; got {}", value.repr()))
                        .at(ctx.caller_span),
                ),
                Err(e) => Ok(EcoString::from(match e.msg {
                    Diagnostic::Error(e) => e.to_string(),
                    Diagnostic::Warning(w) => w.to_string(),
                })),
            }?
        }),
    ])
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
