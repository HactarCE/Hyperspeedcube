use itertools::Itertools;

use crate::{Error, Result, Scope, Warning};

pub fn define_in(scope: &Scope) -> Result<()> {
    scope.register_builtin_functions(hps_fns![
        /// Prints to the output.
        ///
        /// The arguments are converted to strings using `str()` and those
        /// strings are joined with spaces.
        ///
        /// If no arguments are given, prints an empty line.
        fn print(ctx: EvalCtx, args: Args) -> Null {
            ctx.runtime.print(args.iter().join(" "));
        }

        /// Emits a value as a warning message.
        ///
        /// The arguments are converted to strings using `str()` and those
        /// strings are joined with spaces to construct the warning message.
        ///
        /// If no arguments are given, emits a generic warning.
        fn warn(ctx: EvalCtx, args: Args) -> Null {
            ctx.runtime
                .report_diagnostic(Warning::User(args.iter().join(" ").into()).at(ctx.caller_span));
        }

        /// Halts execution with an error message.
        ///
        /// The arguments are converted to strings using `str()` and those
        /// strings are joined with spaces to construct the error message.
        ///
        /// If no arguments are given, emits a generic error.
        fn error(ctx: EvalCtx, args: Args) -> Null {
            let msg = if args.is_empty() {
                "runtime error".into()
            } else {
                args.iter().join(" ").into()
            };
            Err::<(), _>(Error::User(msg).at(ctx.caller_span))?
        }
    ])
}
