use hypermath::{approx_gt, approx_gt_eq, approx_lt, approx_lt_eq};

use crate::{Result, Scope};

pub fn define_in(scope: &Scope) -> Result<()> {
    scope.register_builtin_functions(hps_short_fns![
        ("==", |ctx, a: Any, b: Any| -> Bool {
            a.eq(&b, ctx.caller_span)?
        }),
        ("!=", |ctx, a: Any, b: Any| -> Bool {
            !a.eq(&b, ctx.caller_span)?
        }),
        // Number operators
        ("+", |_, n: Num| -> Num { n }),
        ("-", |_, n: Num| -> Num { -n }),
        ("+", |_, a: Num, b: Num| -> Num { a + b }),
        ("-", |_, a: Num, b: Num| -> Num { a - b }),
        ("*", |_, a: Num, b: Num| -> Num { a * b }),
        ("/", |_, a: Num, b: Num| -> Num { a / b }),
        ("%", |_, a: Num, b: Num| -> Num { a.rem_euclid(b) }),
        ("**", |_, a: Num, b: Num| -> Num { a.powf(b) }),
        ("°", |_, n: Num| -> Num { n.to_radians() }),
        // Number comparisons
        ("<", |_, a: Num, b: Num| -> Bool { approx_lt(&a, &b) }),
        (">", |_, a: Num, b: Num| -> Bool { approx_gt(&a, &b) }),
        ("<=", |_, a: Num, b: Num| -> Bool { approx_lt_eq(&a, &b) }),
        (">=", |_, a: Num, b: Num| -> Bool { approx_gt_eq(&a, &b) }),
    ])
}
