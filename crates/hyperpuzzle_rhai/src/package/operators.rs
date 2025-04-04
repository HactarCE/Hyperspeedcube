//! Overrides for numeric operators that use approximate equality and implicitly
//! convert integers to floats for division.

use hypermath::{approx_eq, approx_gt, approx_gt_eq, approx_lt, approx_lt_eq};

use super::*;

pub fn register(module: &mut Module) {
    // i64 / i64 -> f64
    new_fn("/").set_into_module(module, |a: i64, b: i64| a as f64 / b as f64);

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
}
