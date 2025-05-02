//! Overrides for numeric operators that use approximate equality and implicitly
//! convert integers to floats for division.

use hypermath::{approx_eq, approx_gt, approx_gt_eq, approx_lt, approx_lt_eq};

use super::*;

pub fn register(module: &mut Module) {
    // i64 / i64 -> f64
    new_fn("/").set_into_module(module, |a: i64, b: i64| a as f64 / b as f64);

    // Flooring f64 -> i64
    new_fn("floor").set_into_module(module, |x: f64| -> Result<i64> {
        let x = x.floor();
        if x.is_nan() {
            Err(format!("error: to_int(NaN)").into())
        } else if x > i64::MAX as f64 || x < i64::MAX as f64 {
            Err(format!("integer overflow: to_int({x})").into())
        } else {
            Ok(x as i64)
        }
    });
    new_fn("ceiling").set_into_module(module, |x: f64| -> Result<i64> {
        let x = x.ceil();
        if x.is_nan() {
            Err(format!("error: to_int(NaN)").into())
        } else if x > i64::MAX as f64 || x < i64::MAX as f64 {
            Err(format!("integer overflow: to_int({x})").into())
        } else {
            Ok(x as i64)
        }
    });
    new_fn("round").set_into_module(module, |x: f64| -> Result<i64> {
        let x = x.round();
        if x.is_nan() {
            Err(format!("error: to_int(NaN)").into())
        } else if x > i64::MAX as f64 || x < i64::MAX as f64 {
            Err(format!("integer overflow: to_int({x})").into())
        } else {
            Ok(x as i64)
        }
    });

    // Euclidean `%`
    new_fn("%").set_into_module(module, |a: f64, b: f64| a.rem_euclid(b));
    new_fn("%").set_into_module(module, |a: i64, b: f64| (a as f64).rem_euclid(b));
    new_fn("%").set_into_module(module, |a: f64, b: i64| a.rem_euclid(b as f64));
    new_fn("%").set_into_module(module, |a: i64, b: i64| a.rem_euclid(b));

    // number == number
    new_fn("==").set_into_module(module, |a: f64, b: f64| approx_eq(&a, &b));
    new_fn("==").set_into_module(module, |a: i64, b: f64| approx_eq(&(a as f64), &b));
    new_fn("==").set_into_module(module, |a: f64, b: i64| approx_eq(&a, &(b as f64)));

    // number != number
    new_fn("!=").set_into_module(module, |a: f64, b: f64| !approx_eq(&a, &b));
    new_fn("!=").set_into_module(module, |a: i64, b: f64| !approx_eq(&(a as f64), &b));
    new_fn("!=").set_into_module(module, |a: f64, b: i64| !approx_eq(&a, &(b as f64)));

    // number >= number
    new_fn(">=").set_into_module(module, |a: f64, b: f64| approx_gt_eq(&a, &b));
    new_fn(">=").set_into_module(module, |a: i64, b: f64| approx_gt_eq(&(a as f64), &b));
    new_fn(">=").set_into_module(module, |a: f64, b: i64| approx_gt_eq(&a, &(b as f64)));

    // number > number
    new_fn(">").set_into_module(module, |a: f64, b: f64| approx_gt(&a, &b));
    new_fn(">").set_into_module(module, |a: i64, b: f64| approx_gt(&(a as f64), &b));
    new_fn(">").set_into_module(module, |a: f64, b: i64| approx_gt(&a, &(b as f64)));

    // number <= number
    new_fn("<=").set_into_module(module, |a: f64, b: f64| approx_lt_eq(&a, &b));
    new_fn("<=").set_into_module(module, |a: i64, b: f64| approx_lt_eq(&(a as f64), &b));
    new_fn("<=").set_into_module(module, |a: f64, b: i64| approx_lt_eq(&a, &(b as f64)));

    // number < number
    new_fn("<").set_into_module(module, |a: f64, b: f64| approx_lt(&a, &b));
    new_fn("<").set_into_module(module, |a: i64, b: f64| approx_lt(&(a as f64), &b));
    new_fn("<").set_into_module(module, |a: f64, b: i64| approx_lt(&a, &(b as f64)));

    // math functions on integers
    // from https://rhai.rs/book/ref/num-fn.html#floating-point-functions
    new_fn("sin").set_into_module(module, |a: i64| (a as f64).sin());
    new_fn("cos").set_into_module(module, |a: i64| (a as f64).cos());
    new_fn("tan").set_into_module(module, |a: i64| (a as f64).tan());
    new_fn("sinh").set_into_module(module, |a: i64| (a as f64).sinh());
    new_fn("cosh").set_into_module(module, |a: i64| (a as f64).cosh());
    new_fn("tanh").set_into_module(module, |a: i64| (a as f64).tanh());
    new_fn("hypot").set_into_module(module, |a: f64, b: i64| f64::hypot(a, b as f64));
    new_fn("hypot").set_into_module(module, |a: i64, b: f64| f64::hypot(a as f64, b));
    new_fn("hypot").set_into_module(module, |a: i64, b: i64| f64::hypot(a as f64, b as f64));

    new_fn("asin").set_into_module(module, |a: i64| (a as f64).asin());
    new_fn("acos").set_into_module(module, |a: i64| (a as f64).acos());
    new_fn("atan").set_into_module(module, |a: i64| (a as f64).atan());
    new_fn("atan").set_into_module(module, |a: f64, b: i64| f64::atan2(a, b as f64));
    new_fn("atan").set_into_module(module, |a: i64, b: f64| f64::atan2(a as f64, b));
    new_fn("atan").set_into_module(module, |a: i64, b: i64| f64::atan2(a as f64, b as f64));
    new_fn("asinh").set_into_module(module, |a: i64| (a as f64).asinh());
    new_fn("acosh").set_into_module(module, |a: i64| (a as f64).acosh());
    new_fn("atanh").set_into_module(module, |a: i64| (a as f64).atanh());

    new_fn("sqrt").set_into_module(module, |a: i64| (a as f64).sqrt());

    new_fn("exp").set_into_module(module, |a: i64| (a as f64).exp());

    new_fn("ln").set_into_module(module, |a: i64| (a as f64).ln());
    new_fn("log").set_into_module(module, |a: i64| (a as f64).log10());
    new_fn("log").set_into_module(module, |a: f64, b: i64| a.log(b as f64));
    new_fn("log").set_into_module(module, |a: i64, b: f64| (a as f64).log(b));
    new_fn("log").set_into_module(module, |a: i64, b: i64| (a as f64).log(b as f64));

    new_fn("to_degrees").set_into_module(module, |a: i64| (a as f64).to_degrees());
    new_fn("to_radians").set_into_module(module, |a: i64| (a as f64).to_radians());
}
