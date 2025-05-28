use ecow::eco_format;
use hypermath::approx_cmp;

use crate::{Error, Result, Scope, ValueData};

pub fn define_in(scope: &Scope) -> Result<()> {
    scope.register_builtin_functions([
        // Number functions
        hps_fn!("abs", |x: Num| -> Num { x.abs() }),
        hps_fn!("sign", |x: Num| -> Num {
            match approx_cmp(&x, &0.0) {
                std::cmp::Ordering::Greater => 1.0,
                std::cmp::Ordering::Equal if x.is_sign_positive() => 0.0,
                std::cmp::Ordering::Equal => -0.0,
                std::cmp::Ordering::Less => -1.0,
            }
        }),
        hps_fn!("sqrt", |x: Num| -> Num { x.sqrt() }),
        hps_fn!("√", |x: Num| -> Num { x.sqrt() }),
        hps_fn!("cbrt", |x: Num| -> Num { x.cbrt() }),
        hps_fn!("factorial", |(x, x_span): Int| -> Num {
            if x < 0 {
                return Err(Error::BadArgument {
                    value: ValueData::Num(x as f64).repr(),
                    note: Some("input cannot be negative".to_owned()),
                }
                .at(x_span));
            }
            // convert to float to guard against integer overflow
            (2..=x).map(|x| x as f64).product::<f64>()
        }),
        hps_fn!("is_even", |x: Int| -> Bool { x % 2 == 0 }),
        hps_fn!("is_odd", |x: Int| -> Bool { x % 2 != 0 }),
        hps_fn!("min", |a: Num, b: Num| -> Num { a.min(b) }),
        hps_fn!("max", |a: Num, b: Num| -> Num { a.max(b) }),
        hps_fn!("at_least", |a: Num, b: Num| -> Num { a.max(b) }),
        hps_fn!("at_most", |a: Num, b: Num| -> Num { a.min(b) }),
        hps_fn!("clamp", |ctx, x: Num, bound1: Num, bound2: Num| -> Num {
            match approx_cmp(&bound1, &bound2) {
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
        // Trigonometric functions
        hps_fn!("sin", |x: Num| -> Num { x.sin() }),
        hps_fn!("cos", |x: Num| -> Num { x.cos() }),
        hps_fn!("tan", |x: Num| -> Num { x.tan() }),
        hps_fn!("sinh", |x: Num| -> Num { x.sinh() }),
        hps_fn!("cosh", |x: Num| -> Num { x.cosh() }),
        hps_fn!("tanh", |x: Num| -> Num { x.tanh() }),
        // Inverse trigonometric functions (short names)
        hps_fn!("asin", |x: Num| -> Num { x.asin() }),
        hps_fn!("acos", |x: Num| -> Num { x.acos() }),
        hps_fn!("atan", |x: Num| -> Num { x.atan() }),
        hps_fn!("asinh", |x: Num| -> Num { x.asinh() }),
        hps_fn!("acosh", |x: Num| -> Num { x.acosh() }),
        hps_fn!("atanh", |x: Num| -> Num { x.atanh() }),
        // Inverse trigonometric functions (long names)
        hps_fn!("arcsin", |x: Num| -> Num { x.asin() }),
        hps_fn!("arccos", |x: Num| -> Num { x.acos() }),
        hps_fn!("arctan", |x: Num| -> Num { x.atan() }),
        hps_fn!("arsinh", |x: Num| -> Num { x.asinh() }),
        hps_fn!("arcosh", |x: Num| -> Num { x.acosh() }),
        hps_fn!("artanh", |x: Num| -> Num { x.atanh() }),
        // Reciprocal trigonometric functions
        hps_fn!("csc", |x: Num| -> Num { x.sin().recip() }),
        hps_fn!("sec", |x: Num| -> Num { x.cos().recip() }),
        hps_fn!("cot", |x: Num| -> Num { x.tan().recip() }),
        hps_fn!("csch", |x: Num| -> Num { x.sinh().recip() }),
        hps_fn!("sech", |x: Num| -> Num { x.cosh().recip() }),
        hps_fn!("coth", |x: Num| -> Num { x.tanh().recip() }),
        // Inverse reciprocal trigonometric functions (short names)
        hps_fn!("acsc", |x: Num| -> Num { x.recip().asin() }),
        hps_fn!("asec", |x: Num| -> Num { x.recip().acos() }),
        hps_fn!("acot", |x: Num| -> Num { x.recip().atan() }),
        hps_fn!("acsch", |x: Num| -> Num { x.recip().asinh() }),
        hps_fn!("asech", |x: Num| -> Num { x.recip().acosh() }),
        hps_fn!("acoth", |x: Num| -> Num { x.recip().atanh() }),
        // Inverse reciprocal trigonometric functions (long names)
        hps_fn!("arccsc", |x: Num| -> Num { x.recip().asin() }),
        hps_fn!("arcsec", |x: Num| -> Num { x.recip().acos() }),
        hps_fn!("arccot", |x: Num| -> Num { x.recip().atan() }),
        hps_fn!("arcsch", |x: Num| -> Num { x.recip().asinh() }),
        hps_fn!("arsech", |x: Num| -> Num { x.recip().acosh() }),
        hps_fn!("arcoth", |x: Num| -> Num { x.recip().atanh() }),
        // Exponentials and logarithms
        hps_fn!("exp", |x: Num| -> Num { x.exp() }),
        hps_fn!("exp2", |x: Num| -> Num { x.exp2() }),
        hps_fn!("ln", |x: Num| -> Num { x.ln() }),
        hps_fn!("log2", |x: Num| -> Num { x.log2() }),
        hps_fn!("log10", |x: Num| -> Num { x.log10() }),
        // Rounding
        hps_fn!("round", |x: Num| -> Num { x.round() }),
        hps_fn!("floor", |x: Num| -> Num { x.floor() }),
        hps_fn!("ceil", |x: Num| -> Num { x.ceil() }),
        hps_fn!("ceiling", |x: Num| -> Num { x.ceil() }),
        hps_fn!("trunc", |x: Num| -> Num { x.trunc() }),
        // Infinity
        hps_fn!("is_infinite", |x: Num| -> Num { x.is_infinite() }),
        hps_fn!("is_finite", |x: Num| -> Num { x.is_finite() }),
    ])
}
