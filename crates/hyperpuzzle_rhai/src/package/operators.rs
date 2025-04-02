use super::*;

#[export_module]
pub(super) mod rhai_mod {
    // i64 / i64 -> f64
    #[rhai_fn(global, name = "/")]
    pub fn div_int_int(a: i64, b: i64) -> f64 {
        a as f64 / b as f64
    }
}
