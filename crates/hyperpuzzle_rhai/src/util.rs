use std::fmt;

use rhai::{Dynamic, FuncRegistration};

use crate::{Ctx, Result};

/// Emits a warning.
pub fn warn(ctx: &Ctx<'_>, msg: impl fmt::Display) -> Result {
    ctx.call_native_fn("print", (format!("{msg:#}"),))
}
/// Returns a function that emits a warning and returns a [`Result`].
pub fn warnf<'a, T: fmt::Display>(ctx: &'a Ctx<'_>) -> impl 'a + Copy + Fn(T) -> Result {
    |msg| warn(ctx, msg)
}
/// Returns a function that emits a warning and returns nothing.
pub fn void_warn<'a, T: fmt::Display>(ctx: &'a Ctx<'_>) -> impl 'a + Copy + Fn(T) {
    |msg| {
        let _ = warn(ctx, msg);
    }
}

/// Shortcut for `FuncRegistration::new(name).in_global_namespace()`.
pub fn new_fn(name: &str) -> FuncRegistration {
    FuncRegistration::new(name).in_global_namespace()
}

/// Returns the current number of dimensions.
pub fn get_ndim(ctx: &Ctx<'_>) -> Result<u8> {
    // TODO: make this work
    ctx.call_native_fn("_get_ndim", ())
}

/// Calls Rhai `to_string()` on `val` and returns the result.
pub fn rhai_to_string(ctx: &Ctx<'_>, val: &Dynamic) -> String {
    ctx.call_fn::<String>(rhai::FUNC_TO_STRING, (val.clone(),))
        .unwrap_or_else(|_| val.to_string())
}

/// Calls Rhai `to_debug()` on `val` and returns the result.
pub fn rhai_to_debug(ctx: &Ctx<'_>, val: &Dynamic) -> String {
    ctx.call_fn::<String>(rhai::FUNC_TO_DEBUG, (val.clone(),))
        .unwrap_or_else(|_| val.to_string())
}
