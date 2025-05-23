use std::sync::Arc;

use ecow::{EcoString, eco_format};
use hypermath::{approx_gt, approx_gt_eq, approx_lt, approx_lt_eq};
use itertools::Itertools;
use smallvec::smallvec;

use crate::{Diagnostic, Error, Result, Scope, Span, Type, Value, Warning};

pub fn new_builtins_scope() -> Arc<Scope> {
    let scope = Scope::new();
    add_builtin_functions(&scope).expect("error adding built-in functions");
    scope
}

macro_rules! hps_fn {
    ($fn_name:expr, || $($rest:tt)*) => {
        hps_fn!($fn_name, |_ctx| $($rest)*)
    };
    ($fn_name:expr, |$($param:tt : $param_ty:ident $( ( $($param_ty_contents:tt)* ) )?),*| -> $ret_ty:ident { $($body:tt)* }) => {
        hps_fn!($fn_name, |_ctx, $($param : $param_ty $( ( $($param_ty_contents)* ) )?),*| -> $ret_ty { $($body)* })
    };
    ($fn_name:expr, |$ctx:ident $(, $param:tt : $param_ty:ident $( ( $($param_ty_contents:tt)* ) )?)* $(,)?| -> $ret_ty:ident { $($body:tt)* }) => {
        hps_fn!(
            $fn_name,
            (
                Some(vec![$(ty_from_tokens!($param_ty $( ( $($param_ty_contents)* ) )?)),*]),
                $crate::Type::$ret_ty
            ),
            |$ctx, args| {
                #[allow(unused)]
                let mut args = args;
                #[allow(unused)]
                let mut i = 0;
                $(
                    unpack_val!(let $param = std::mem::take(&mut args[i]), $param_ty $( ( $($param_ty_contents)* ) )?);
                    #[allow(unused_assignments)]
                    {
                        i += 1;
                    }
                )*
                $($body)*
            }
        )
    };
    ($fn_name:expr, ($params:expr, $ret:expr $(,)?), || -> $($rest:tt)*) => {
        hps_fn!($fn_name, ($params, $ret), | | -> $($rest)*)
    };
    ($fn_name:expr, ($params:expr, $ret:expr $(,)?), |$args:ident $(,)?| { $($body:tt)* }) => {
        hps_fn!($fn_name, ($params, $ret), |_ctx, $args| { $($body)* })
    };
    ($fn_name:expr, ($params:expr, $ret:expr $(,)?), |$ctx:ident, $args:ident $(,)?| { $($body:tt)* }) => {
        (
            $fn_name,
            $crate::FnOverload {
                ty: $crate::FnType { params: $params, ret: $ret },
                call: Arc::new(|$ctx, $args| {
                    let output = { $($body)* };
                    Ok($crate::ValueData::from(output).at($crate::BUILTIN_SPAN))
                }),
                debug_info: $crate::FnDebugInfo::Internal($fn_name),
                new_scope: false, // built-in functions never modify local variables
            },
        )
    };
    (|$($param:tt : $param_ty:ident),*| $body:expr) => {
        compile_error!("missing return type")
    };
    (|$($param:tt $(: $param_ty:ident)?),*| $($rest:tt)*) => {
        compile_error!("missing argument type")
    };
}

macro_rules! ty_from_tokens {
    (Fn) => {
        $crate::Type::Fn(std::default::Default::default())
    };
    (List) => {
        $crate::Type::List(std::boxed::Box::new($crate::Type::Any))
    };
    (Map) => {
        $crate::Type::Map(std::boxed::Box::new($crate::Type::Any))
    };
    ($collection_ty:ident ( $($inner:tt)* )) => {
        $crate::Type::$collection_ty(std::boxed::Box::new(ty_from_tokens!($($inner)*)))
    };
    ($($tok:tt)*) => { $crate::Type::$($tok)* };
}

macro_rules! unpack_val {
    (let $dst:ident = $($rest:tt)*) => { unpack_val!(let ($dst, _) = $($rest)*) };
    (let ($dst:ident, $span:tt) = $val:expr, $($ty:tt)*) => {
        let val = $val;
        let $span = val.span;
        let $dst = unpack_val!(val, $($ty)*);
    };

    ($val:ident, Any)  => { $val };
    ($val:ident, Null) => { unpack_val!(@$val, (Null), $crate::ValueData::Null => ()) };
    ($val:ident, Bool) => { unpack_val!(@$val, (Bool), $crate::ValueData::Bool(b) => b) };
    ($val:ident, Num)  => { unpack_val!(@$val, (Num),  $crate::ValueData::Num(n) => n) };
    ($val:ident, Str)  => { unpack_val!(@$val, (Str),  $crate::ValueData::Str(s) => s) };
    ($val:ident, List) => { unpack_val!(@$val, (List), $crate::ValueData::List(l) => l) };
    ($val:ident, Map)  => { unpack_val!(@$val, (Map),  $crate::ValueData::Map(m) => m) };
    ($val:ident, Fn)   => { unpack_val!(@$val, (Fn),   $crate::ValueData::Fn(f) => f) };
    ($val:ident, Vec)  => { unpack_val!(@$val, (Vec),  $crate::ValueData::Vec(v) => v) };
    // TODO: more types, including integers
    ($val:ident, $other:ident) => { compile_error!(concat!("unsupported type: ", stringify!($other))) };

    // Collection types
    ($val:ident, List ( $($inner:tt)* )) => {
        unpack_val!(@$val, (List ( $($inner)* )), $crate::ValueData::List(l) => {
            std::sync::Arc::unwrap_or_clone(l)
                .into_iter()
                .map(|elem| {
                    unpack_val!(let e = elem, $($inner)*);
                    Ok(e)
                })
                .collect::<std::result::Result<Vec<_>, _>>()?
        })
    };

    (@$val:ident, ($($expected_ty:tt)*), $pattern:pat => $ret:expr) => {
        match $val.data {
            $pattern => $ret,
            _ => return Err($val.type_error(ty_from_tokens!($($expected_ty)*))),
        }
    };
}

pub fn add_builtin_functions(scope: &Scope) -> Result<()> {
    scope.register_builtin_functions([
        // String conversion
        hps_fn!("str", |arg: Any| -> Str { eco_format!("{arg}") }),
        hps_fn!("repr", |arg: Any| -> Str { eco_format!("{:?}", arg.data) }),
        // Number operators
        hps_fn!("+", |n: Num| -> Num { n }),
        hps_fn!("-", |n: Num| -> Num { -n }),
        hps_fn!("+", |a: Num, b: Num| -> Num { a + b }),
        hps_fn!("-", |a: Num, b: Num| -> Num { a - b }),
        hps_fn!("*", |a: Num, b: Num| -> Num { a * b }),
        hps_fn!("/", |a: Num, b: Num| -> Num { a / b }),
        hps_fn!("%", |a: Num, b: Num| -> Num { a.rem_euclid(b) }),
        hps_fn!("**", |a: Num, b: Num| -> Num { a.powf(b) }),
        hps_fn!("sqrt", |x: Num| -> Num { x.sqrt() }),
        // General comparisons
        hps_fn!("==", |ctx, a: Any, b: Any| -> Bool {
            a.eq(&b, ctx.caller_span)?
        }),
        hps_fn!("!=", |ctx, a: Any, b: Any| -> Bool {
            !a.eq(&b, ctx.caller_span)?
        }),
        // Number comparisons
        hps_fn!("<", |a: Num, b: Num| -> Bool { approx_lt(&a, &b) }),
        hps_fn!(">", |a: Num, b: Num| -> Bool { approx_gt(&a, &b) }),
        hps_fn!("<=", |a: Num, b: Num| -> Bool { approx_lt_eq(&a, &b) }),
        hps_fn!(">=", |a: Num, b: Num| -> Bool { approx_gt_eq(&a, &b) }),
        // Output
        hps_fn!("print", (None, Type::Null), |ctx, args| {
            ctx.runtime.print(args.iter().join(" "));
        }),
        hps_fn!("warn", (None, Type::Null), |ctx, args| {
            ctx.runtime
                .report_diagnostic(Warning::User(args.iter().join(" ").into()).at(ctx.caller_span));
        }),
        hps_fn!("error", (None, Type::Null), |ctx, args| {
            let msg = if args.is_empty() {
                "runtime error".into()
            } else {
                args.iter().join(" ").into()
            };
            Err::<(), _>(Error::User(msg).at(ctx.caller_span))?
        }),
        // Assertions
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
        // Vector construction
        hps_fn!("vec", |nums: List(Num)| -> Vec {
            hypermath::Vector(nums.into())
        }),
        hps_fn!("vec", || -> Vec { hypermath::Vector(smallvec![]) }),
        hps_fn!("vec", |x: Num| -> Vec { hypermath::Vector(smallvec![x]) }),
        hps_fn!("vec", |x: Num, y: Num| -> Vec {
            hypermath::Vector(smallvec![x, y])
        }),
        hps_fn!("vec", |x: Num, y: Num, z: Num| -> Vec {
            hypermath::Vector(smallvec![x, y, z])
        }),
        hps_fn!("vec", |x: Num, y: Num, z: Num, w: Num| -> Vec {
            hypermath::Vector(smallvec![x, y, z, w])
        }),
        hps_fn!("vec", |x: Num, y: Num, z: Num, w: Num, v: Num| -> Vec {
            hypermath::Vector(smallvec![x, y, z, w, v])
        }),
        hps_fn!("vec", |x: Num,
                        y: Num,
                        z: Num,
                        w: Num,
                        v: Num,
                        u: Num|
         -> Vec {
            hypermath::Vector(smallvec![x, y, z, w, v, u])
        }),
        hps_fn!("vec", |x: Num,
                        y: Num,
                        z: Num,
                        w: Num,
                        v: Num,
                        u: Num,
                        t: Num|
         -> Vec {
            hypermath::Vector(smallvec![x, y, z, w, v, u, t])
        }),
    ])?;

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
