use std::borrow::Cow;

use hypermath::Vector;

use super::*;

pub fn warn(ctx: &Ctx<'_>, msg: impl Into<Cow<'static, str>>) -> Result {
    ctx.call_native_fn("print", (msg.into(),))
}

pub fn try_as_number(v: &Dynamic) -> Result<f64> {
    v.as_float()
        .or_else(|_| Ok(v.as_int()? as f64))
        .map_err(|ty: &str| format!("expected number; got {ty}").into())
}

/// Shortcut for `FuncRegistration::new(name).in_global_namespace()`.
pub fn new_fn(name: &str) -> FuncRegistration {
    FuncRegistration::new(name).in_global_namespace()
}

pub fn get_ndim(ctx: Ctx<'_>) -> Result<u8> {
    ctx.call_native_fn("_get_ndim", ())
}

pub fn expected<'a>(expected: &'a str) -> impl 'a + Fn(Dynamic) -> String {
    move |value| expected_ref(expected)(&value)
}
pub fn expected_ref<'a>(expected: &'a str) -> impl 'a + Fn(&Dynamic) -> String {
    move |value| format!("expected {expected}; got {}", value.type_name())
}
pub fn expected_ref_value<'a>(
    ctx: &'a Ctx<'_>,
    expected: &'a str,
) -> impl 'a + Fn(&Dynamic) -> String {
    move |value| {
        format!(
            "expected {expected}; got {} {}",
            value.type_name(),
            rhai_to_debug(ctx, value),
        )
    }
}

pub fn try_collect_to_vector(values: &[Dynamic]) -> Result<Vector> {
    values
        .iter()
        .map(util::try_as_number)
        .collect::<Result<Vector, _>>()
}
pub fn try_collect_to_point(values: &[Dynamic]) -> Result<Point> {
    try_collect_to_vector(values).map(Point)
}
pub fn try_set_vector_component(vector: &mut Vector, axis: i64, new_value: f64) -> Result {
    if (0..hypermath::MAX_NDIM as i64).contains(&axis) {
        vector.resize_and_set(axis as u8, new_value);
        Ok(())
    } else {
        Err(format!("bad vector index {axis}").into())
    }
}

pub fn rhai_to_string(ctx: &Ctx<'_>, val: &Dynamic) -> String {
    ctx.call_fn::<String>(rhai::FUNC_TO_STRING, (val.clone(),))
        .unwrap_or_else(|_| val.to_string())
}

pub fn rhai_to_debug(ctx: &Ctx<'_>, val: &Dynamic) -> String {
    ctx.call_fn::<String>(rhai::FUNC_TO_DEBUG, (val.clone(),))
        .unwrap_or_else(|_| val.to_string())
}
