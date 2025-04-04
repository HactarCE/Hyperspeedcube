use super::*;

#[export_module]
pub(super) mod rhai_mod {
    // i64 / i64 -> f64
    #[rhai_fn(global, name = "/")]
    pub fn div_int_int(a: i64, b: i64) -> f64 {
        a as f64 / b as f64
    }

    // number == number
    #[rhai_fn(global, name = "==")]
    pub fn eq_float_float(a: f64, b: f64) -> bool {
        hypermath::approx_eq(&a, &b)
    }
    #[rhai_fn(global, name = "==")]
    pub fn eq_int_float(a: i64, b: f64) -> bool {
        hypermath::approx_eq(&(a as f64), &b)
    }
    #[rhai_fn(global, name = "==")]
    pub fn eq_float_int(a: f64, b: i64) -> bool {
        hypermath::approx_eq(&a, &(b as f64))
    }

    // number != number
    #[rhai_fn(global, name = "!=")]
    pub fn neq_float_float(a: f64, b: f64) -> bool {
        !hypermath::approx_eq(&a, &b)
    }
    #[rhai_fn(global, name = "!=")]
    pub fn neq_int_float(a: i64, b: f64) -> bool {
        !hypermath::approx_eq(&(a as f64), &b)
    }
    #[rhai_fn(global, name = "!=")]
    pub fn neq_float_int(a: f64, b: i64) -> bool {
        !hypermath::approx_eq(&a, &(b as f64))
    }

    // number >= number
    #[rhai_fn(global, name = ">=")]
    pub fn gte_float_float(a: f64, b: f64) -> bool {
        hypermath::approx_gt_eq(&a, &b)
    }
    #[rhai_fn(global, name = ">=")]
    pub fn gte_int_float(a: i64, b: f64) -> bool {
        hypermath::approx_gt_eq(&(a as f64), &b)
    }
    #[rhai_fn(global, name = ">=")]
    pub fn gte_float_int(a: f64, b: i64) -> bool {
        hypermath::approx_gt_eq(&a, &(b as f64))
    }

    // number > number
    #[rhai_fn(global, name = ">")]
    pub fn gt_float_float(a: f64, b: f64) -> bool {
        hypermath::approx_gt(&a, &b)
    }
    #[rhai_fn(global, name = ">")]
    pub fn gt_int_float(a: i64, b: f64) -> bool {
        hypermath::approx_gt(&(a as f64), &b)
    }
    #[rhai_fn(global, name = ">")]
    pub fn gt_float_int(a: f64, b: i64) -> bool {
        hypermath::approx_gt(&a, &(b as f64))
    }

    // number <= number
    #[rhai_fn(global, name = "<=")]
    pub fn lte_float_float(a: f64, b: f64) -> bool {
        hypermath::approx_lt_eq(&a, &b)
    }
    #[rhai_fn(global, name = "<=")]
    pub fn lte_int_float(a: i64, b: f64) -> bool {
        hypermath::approx_lt_eq(&(a as f64), &b)
    }
    #[rhai_fn(global, name = "<=")]
    pub fn lte_float_int(a: f64, b: i64) -> bool {
        hypermath::approx_lt_eq(&a, &(b as f64))
    }

    // number < number
    #[rhai_fn(global, name = "<")]
    pub fn lt_float_float(a: f64, b: f64) -> bool {
        hypermath::approx_lt(&a, &b)
    }
    #[rhai_fn(global, name = "<")]
    pub fn lt_int_float(a: i64, b: f64) -> bool {
        hypermath::approx_lt(&(a as f64), &b)
    }
    #[rhai_fn(global, name = "<")]
    pub fn lt_float_int(a: f64, b: i64) -> bool {
        hypermath::approx_lt(&a, &(b as f64))
    }
}
