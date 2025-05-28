use hypermath::{approx_gt, approx_gt_eq, approx_lt, approx_lt_eq};

use crate::{Result, Scope};

pub fn define_in(scope: &Scope) -> Result<()> {
    scope.register_builtin_functions([
        // General comparisons
        hps_fn!("==", |ctx, a: Any, b: Any| -> Bool {
            a.eq(&b, ctx.caller_span)?
        }),
        hps_fn!("!=", |ctx, a: Any, b: Any| -> Bool {
            !a.eq(&b, ctx.caller_span)?
        }),
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
        // Number comparisons
        hps_fn!("<", |a: Num, b: Num| -> Bool { approx_lt(&a, &b) }),
        hps_fn!(">", |a: Num, b: Num| -> Bool { approx_gt(&a, &b) }),
        hps_fn!("<=", |a: Num, b: Num| -> Bool { approx_lt_eq(&a, &b) }),
        hps_fn!(">=", |a: Num, b: Num| -> Bool { approx_gt_eq(&a, &b) }),
    ])
}
