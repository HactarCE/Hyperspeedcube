//! Basic non-mathematical operators `==` and `!=`.

use crate::{Result, Scope, Value};

/// Adds the built-in operators to the scope.
pub fn define_in(scope: &Scope) -> Result<()> {
    scope.register_builtin_functions(hps_fns![
        ("==", |ctx, a: Value, b: Value| -> bool {
            a.eq(&b, ctx.caller_span)?
        }),
        ("!=", |ctx, a: Value, b: Value| -> bool {
            !a.eq(&b, ctx.caller_span)?
        }),
    ])
}
