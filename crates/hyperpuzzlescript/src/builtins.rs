use arcstr::Substr;
use ecow::eco_format;
use hypermath::{approx_eq, approx_gt, approx_gt_eq, approx_lt, approx_lt_eq};
use std::{io::Write, sync::Arc};

use crate::{FnOverload, FnType, FnValue, Result, Scope, Type, Value, ValueData};

pub fn new_builtins_scope() -> Arc<Scope> {
    let scope = Scope::new();
    add_builtin_functions(&scope).expect("error adding built-in functions");
    scope
}

macro_rules! hps_fn {
    ($fn_name:expr, |$($param:tt : $param_ty:ident),*| -> $ret_ty:ident { $($body:tt)* }) => {
        (
            $fn_name,
            $crate::FnOverload {
                ty: $crate::FnType {
                    params: Some(vec![$($crate::Type::$param_ty),*]),
                    ret: $crate::Type::$ret_ty,
                },
                call: Arc::new(|#[allow(unused)] ctx, mut args| {
                    #[allow(unused)]
                    let mut i = 0;
                    $(
                        unpack_val!(let $param = $param_ty, std::mem::take(&mut args[i]));
                        #[allow(unused_assignments)]
                        {
                            i += 1;
                        }
                    )*
                    let result = $($body)*;
                    Ok($crate::ValueData::from(result).at($crate::BUILTIN_SPAN))
                }),
                debug_info: $crate::FnDebugInfo::Internal($fn_name),
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

macro_rules! unpack_val {
    (let $dst:ident = $($rest:tt)*) => { unpack_val!(let ($dst, _) = $($rest)*) };
    (let ($dst:ident, $span:tt) = $ty:ident, $val:expr) => {
        let val = $val;
        let $span = val.span;
        let $dst = unpack_val!($ty, val);
    };

    (Any,    $val:ident) => { $val };
    (Null,   $val:ident) => { unpack_val!(@$val, $crate::ValueData::Null => ()) };
    (Bool,   $val:ident) => { unpack_val!(@$val, $crate::ValueData::Bool(b) => b) };
    (Num,    $val:ident) => { unpack_val!(@$val, $crate::ValueData::Num(n) => n) };
    (Str,    $val:ident) => { unpack_val!(@$val, $crate::ValueData::Str(s) => s) };
    (List,   $val:ident) => { unpack_val!(@$val, $crate::ValueData::List(l) => l) };
    (Map,    $val:ident) => { unpack_val!(@$val, $crate::ValueData::Map(m) => m) };
    (Fn,     $val:ident) => { unpack_val!(@$val, $crate::ValueData::Fn(f) => f) };
    (Vector, $val:ident) => { unpack_val!(@$val, $crate::ValueData::Vector(v) => v) };
    // TODO: more types, including integers
    ($other:ident, $val:ident) => { compile_error!(concat!("unsupported type: ", $other)) };

    (@$val:ident, $pattern:pat => $ret:expr) => {
        match $val.data {
            $pattern => $ret,
            _ => unreachable!("uncaught type error"),
        }
    };
}

pub fn add_builtin_functions(scope: &Scope) -> Result<()> {
    scope.register_builtin_functions([
        hps_fn!("str", |arg: Any| -> Str { eco_format!("{}", arg) }),
        // Number operators
        hps_fn!("+", |a: Num, b: Num| -> Num { a + b }),
        hps_fn!("-", |a: Num, b: Num| -> Num { a - b }),
        hps_fn!("*", |a: Num, b: Num| -> Num { a * b }),
        hps_fn!("/", |a: Num, b: Num| -> Num { a / b }),
        hps_fn!("%", |a: Num, b: Num| -> Num { a.rem_euclid(b) }),
        hps_fn!("**", |a: Num, b: Num| -> Num { a.powf(b) }),
        // Number comparisons
        hps_fn!("==", |a: Num, b: Num| -> Bool { approx_eq(&a, &b) }),
        hps_fn!("!=", |a: Num, b: Num| -> Bool { !approx_eq(&a, &b) }),
        hps_fn!("<", |a: Num, b: Num| -> Bool { approx_lt(&a, &b) }),
        hps_fn!(">", |a: Num, b: Num| -> Bool { approx_gt(&a, &b) }),
        hps_fn!("<=", |a: Num, b: Num| -> Bool { approx_lt_eq(&a, &b) }),
        hps_fn!(">=", |a: Num, b: Num| -> Bool { approx_gt_eq(&a, &b) }),
        // String comparisons
        hps_fn!("==", |a: Str, b: Str| -> Bool { a == b }),
        hps_fn!("!=", |a: Str, b: Str| -> Bool { a != b }),
    ])?;

    // scope.set(
    //     "^".into(),
    //     Value {
    //         data: ValueData::Fn(Arc::new(FnValue::default())),
    //         span: crate::BUILTIN_SPAN,
    //     },
    // );

    scope.register_func(
        crate::BUILTIN_SPAN,
        Substr::from("print"),
        FnOverload {
            ty: FnType {
                params: None,
                ret: Type::Null,
            },
            call: Arc::new(|_ctx, args| {
                let mut stdout = std::io::stdout().lock();
                let mut is_first = true;
                for arg in args {
                    if is_first {
                        is_first = false;
                    } else {
                        write!(stdout, " ").unwrap();
                    }
                    write!(stdout, "{}", arg.to_string()).unwrap();
                }
                writeln!(stdout).unwrap();
                Ok(ValueData::Null.at(crate::BUILTIN_SPAN))
            }),
            debug_info: crate::FnDebugInfo::Internal("print"),
        },
    )?;

    Ok(())
}
