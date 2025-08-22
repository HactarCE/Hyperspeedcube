//! Mathematical constants, operators, and functions.

use std::f64::consts::{PI, TAU};

use ecow::eco_format;
use hypermath::APPROX;

use crate::{Builtins, Error, Num, Result};

const PHI: f64 = 1.618_033_988_749_895_f64;

/// Adds the built-in constants, operators, and functions.
pub fn define_in(builtins: &mut Builtins<'_>) -> Result<()> {
    // Constants
    builtins.set("pi", PI)?;
    builtins.set("π", PI)?;
    builtins.set("tau", TAU)?;
    builtins.set("τ", TAU)?;
    builtins.set("phi", PHI)?;
    builtins.set("φ", PHI)?;
    builtins.set("deg", 1.0_f64.to_radians())?;
    builtins.set("inf", f64::INFINITY)?;
    builtins.set("∞", f64::INFINITY)?;

    // Operators
    builtins.set_fns(hps_fns![
        // Number operators
        ("+", |_, n: Num| -> Num { n }),
        ("-", |_, n: Num| -> Num { -n }),
        ("+", |_, a: Num, b: Num| -> Num { a + b }),
        ("-", |_, a: Num, b: Num| -> Num { a - b }),
        ("*", |_, a: Num, b: Num| -> Num { a * b }),
        ("/", |_, a: Num, b: Num| -> Num { a / b }),
        ("%", |_, a: Num, b: Num| -> Num { a.rem_euclid(b) }),
        ("^", |_, a: Num, b: Num| -> Num { a.powf(b) }),
        ("°", |_, n: Num| -> Num { n.to_radians() }),
        // Number comparisons
        ("<", |_, a: Num, b: Num| -> bool { APPROX.lt(a, b) }),
        (">", |_, a: Num, b: Num| -> bool { APPROX.gt(a, b) }),
        ("<=", |_, a: Num, b: Num| -> bool { APPROX.lt_eq(a, b) }),
        (">=", |_, a: Num, b: Num| -> bool { APPROX.gt_eq(a, b) }),
    ])?;

    builtins.set_fns(hps_fns![
        /// `sqrt()` returns the square root of a number.
        ///
        /// You can also use the Unicode symbol `√` as a prefix operator to get
        /// the square root of an expression.
        ///
        /// ```title="Examples using sqrt()"
        /// assert_eq(sqrt(3)/2, (3/4) ** (1/2))
        /// assert_eq(√2, 2.sqrt())
        /// ```
        fn sqrt(n: Num) -> Num {
            n.sqrt()
        }
    ])?;
    builtins.set_fns(hps_fns![("√", |_ctx, x: Num| -> Num { x.sqrt() })])?;

    builtins.set_fns(hps_fns![
        // Number functions
        ("abs", |_, x: Num| -> Num { x.abs() }),
        ("sign", |_, x: Num| -> Num {
            match APPROX.cmp_zero(x) {
                std::cmp::Ordering::Greater => 1.0,
                std::cmp::Ordering::Equal if x.is_sign_positive() => 0.0,
                std::cmp::Ordering::Equal => -0.0,
                std::cmp::Ordering::Less => -1.0,
            }
        }),
        ("cbrt", |_, x: Num| -> Num { x.cbrt() }),
        ("factorial", |_, (x, x_span): i64| -> Num {
            if x < 0 {
                let msg = "input cannot be negative";
                return Err(Error::bad_arg(x as f64, Some(msg)).at(x_span));
            }
            // convert to float to guard against integer overflow
            (2..=x).map(|x| x as f64).product::<f64>()
        }),
        ("is_even", |_, x: i64| -> bool { x % 2 == 0 }),
        ("is_odd", |_, x: i64| -> bool { x % 2 != 0 }),
        ("min", |_, a: Num, b: Num| -> Num { a.min(b) }),
        ("max", |_, a: Num, b: Num| -> Num { a.max(b) }),
        ("at_least", |_, a: Num, b: Num| -> Num { a.max(b) }),
        ("at_most", |_, a: Num, b: Num| -> Num { a.min(b) }),
        ("clamp", |ctx, x: Num, bound1: Num, bound2: Num| -> Num {
            match APPROX.cmp(bound1, bound2) {
                std::cmp::Ordering::Less => x.clamp(bound1, bound2),
                std::cmp::Ordering::Equal => bound1,
                std::cmp::Ordering::Greater => {
                    return Err(Error::User(eco_format!(
                        "bounds are out of order: {bound1}, {bound2}"
                    ))
                    .at(ctx.caller_span));
                }
            }
        }),
        // Interpolation
        ("lerp", |_, a: Num, b: Num, t: Num| -> Num {
            let t = t.clamp(0.0, 1.0);
            a * (1.0 - t) + b * t
        }),
        ("lerp_unbounded", |_, a: Num, b: Num, t: Num| -> Num {
            a * (1.0 - t) + b * t
        }),
        // Trigonometric functions
        ("sin", |_, x: Num| -> Num { x.sin() }),
        ("cos", |_, x: Num| -> Num { x.cos() }),
        ("tan", |_, x: Num| -> Num { x.tan() }),
        ("sinh", |_, x: Num| -> Num { x.sinh() }),
        ("cosh", |_, x: Num| -> Num { x.cosh() }),
        ("tanh", |_, x: Num| -> Num { x.tanh() }),
        // Inverse trigonometric functions (short names)
        ("asin", |_, x: Num| -> Num { x.asin() }),
        ("acos", |_, x: Num| -> Num { x.acos() }),
        ("atan", |_, x: Num| -> Num { x.atan() }),
        ("asinh", |_, x: Num| -> Num { x.asinh() }),
        ("acosh", |_, x: Num| -> Num { x.acosh() }),
        ("atanh", |_, x: Num| -> Num { x.atanh() }),
        // Inverse trigonometric functions (long names)
        ("arcsin", |_, x: Num| -> Num { x.asin() }),
        ("arccos", |_, x: Num| -> Num { x.acos() }),
        ("arctan", |_, x: Num| -> Num { x.atan() }),
        ("arsinh", |_, x: Num| -> Num { x.asinh() }),
        ("arcosh", |_, x: Num| -> Num { x.acosh() }),
        ("artanh", |_, x: Num| -> Num { x.atanh() }),
        // Reciprocal trigonometric functions
        ("csc", |_, x: Num| -> Num { x.sin().recip() }),
        ("sec", |_, x: Num| -> Num { x.cos().recip() }),
        ("cot", |_, x: Num| -> Num { x.tan().recip() }),
        ("csch", |_, x: Num| -> Num { x.sinh().recip() }),
        ("sech", |_, x: Num| -> Num { x.cosh().recip() }),
        ("coth", |_, x: Num| -> Num { x.tanh().recip() }),
        // Inverse reciprocal trigonometric functions (short names)
        ("acsc", |_, x: Num| -> Num { x.recip().asin() }),
        ("asec", |_, x: Num| -> Num { x.recip().acos() }),
        ("acot", |_, x: Num| -> Num { x.recip().atan() }),
        ("acsch", |_, x: Num| -> Num { x.recip().asinh() }),
        ("asech", |_, x: Num| -> Num { x.recip().acosh() }),
        ("acoth", |_, x: Num| -> Num { x.recip().atanh() }),
        // Inverse reciprocal trigonometric functions (long names)
        ("arccsc", |_, x: Num| -> Num { x.recip().asin() }),
        ("arcsec", |_, x: Num| -> Num { x.recip().acos() }),
        ("arccot", |_, x: Num| -> Num { x.recip().atan() }),
        ("arcsch", |_, x: Num| -> Num { x.recip().asinh() }),
        ("arsech", |_, x: Num| -> Num { x.recip().acosh() }),
        ("arcoth", |_, x: Num| -> Num { x.recip().atanh() }),
        // Exponentials and logarithms
        ("exp", |_, x: Num| -> Num { x.exp() }),
        ("exp2", |_, x: Num| -> Num { x.exp2() }),
        ("ln", |_, x: Num| -> Num { x.ln() }),
        ("log2", |_, x: Num| -> Num { x.log2() }),
        ("log10", |_, x: Num| -> Num { x.log10() }),
        // Rounding
        ("round", |_, x: Num| -> Num { x.round() }),
        ("floor", |_, x: Num| -> Num { x.floor() }),
        ("ceil", |_, x: Num| -> Num { x.ceil() }),
        ("ceiling", |_, x: Num| -> Num { x.ceil() }),
        ("trunc", |_, x: Num| -> Num { x.trunc() }),
        // Infinity
        ("is_infinite", |_, x: Num| -> bool { x.is_infinite() }),
        ("is_finite", |_, x: Num| -> bool { x.is_finite() }),
    ])
}
