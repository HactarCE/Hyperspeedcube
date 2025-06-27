//! Output functions `print()`, `warn()`, and `error()`.

use itertools::Itertools;

use crate::{Builtins, Error, Result, Warning};

/// Adds the built-in functions.
pub fn define_in(builtins: &mut Builtins<'_>) -> Result<()> {
    builtins.set_fns(hps_fns![
        /// Prints to the output.
        ///
        /// The arguments are converted to strings using `str()` and those
        /// strings are joined with spaces.
        ///
        /// If no arguments are given, prints an empty line.
        fn print(ctx: EvalCtx, args: Args) -> () {
            ctx.runtime.print(args.iter().join(" "));
        }

        /// Emits a value as a warning message.
        ///
        /// The arguments are converted to strings using `str()` and those
        /// strings are joined with spaces to construct the warning message.
        ///
        /// If no arguments are given, emits a generic warning.
        fn warn(ctx: EvalCtx, args: Args) -> () {
            ctx.runtime
                .report_diagnostic(Warning::User(args.iter().join(" ").into()).at(ctx.caller_span));
        }

        /// Halts execution with an error message.
        ///
        /// The arguments are converted to strings using `str()` and those
        /// strings are joined with spaces to construct the error message.
        ///
        /// If no arguments are given, emits a generic error.
        fn error(ctx: EvalCtx, args: Args) -> () {
            let msg = if args.is_empty() {
                "runtime error".into()
            } else {
                args.iter().join(" ").into()
            };
            Err::<(), _>(Error::User(msg).at(ctx.caller_span))?;
        }
    ])
}
