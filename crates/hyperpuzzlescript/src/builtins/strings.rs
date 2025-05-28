use crate::{Result, Scope};

pub fn add_builtin_functions(scope: &Scope) -> Result<()> {
    scope.register_builtin_functions([
        // String conversion
        hps_fn!("str", |arg: Any| -> Str { eco_format!("{arg}") }),
        hps_fn!("repr", |arg: Any| -> Str { eco_format!("{:?}", arg.data) }),
    ])
}
