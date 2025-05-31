use itertools::Itertools;

use crate::{Error, Result, Scope, Type, Warning};

pub fn define_in(scope: &Scope) -> Result<()> {
    scope.register_builtin_functions([
        hps_fn!("print", (None, Type::Null), |ctx, args, kwargs| {
            unpack_kwargs!(kwargs);
            ctx.runtime.print(args.iter().join(" "));
        }),
        hps_fn!("warn", (None, Type::Null), |ctx, args, kwargs| {
            unpack_kwargs!(kwargs);
            ctx.runtime
                .report_diagnostic(Warning::User(args.iter().join(" ").into()).at(ctx.caller_span));
        }),
        hps_fn!("error", (None, Type::Null), |ctx, args, kwargs| {
            unpack_kwargs!(kwargs);
            let msg = if args.is_empty() {
                "runtime error".into()
            } else {
                args.iter().join(" ").into()
            };
            Err::<(), _>(Error::User(msg).at(ctx.caller_span))?
        }),
    ])
}
