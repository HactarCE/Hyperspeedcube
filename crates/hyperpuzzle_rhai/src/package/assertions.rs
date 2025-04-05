//! Assertion functions.

use super::*;

pub fn register(module: &mut Module) {
    new_fn("assert").set_into_module(
        module,
        |ctx: NativeCallContext<'_>, condition: bool| -> Result {
            assert(&ctx, condition, || "assertion error")
        },
    );
    new_fn("assert").set_into_module(
        module,
        |ctx: NativeCallContext<'_>, condition: bool, msg: Dynamic| -> Result {
            assert(&ctx, condition, || msg)
        },
    );

    new_fn("assert_eq").set_into_module(
        module,
        |ctx: NativeCallContext<'_>, val1: Dynamic, val2: Dynamic| -> Result {
            assert_eq(&ctx, &val1, &val2, &"assertion failed".into())
        },
    );
    new_fn("assert_eq").set_into_module(
        module,
        |ctx: NativeCallContext<'_>, val1: Dynamic, val2: Dynamic, msg: Dynamic| -> Result {
            assert_eq(&ctx, &val1, &val2, &msg)
        },
    );

    new_fn("assert_neq").set_into_module(
        module,
        |ctx: NativeCallContext<'_>, val1: Dynamic, val2: Dynamic| -> Result {
            assert_neq(&ctx, &val1, &val2, &"assertion failed".into())
        },
    );
    new_fn("assert_neq").set_into_module(
        module,
        |ctx: NativeCallContext<'_>, val1: Dynamic, val2: Dynamic, msg: Dynamic| -> Result {
            assert_neq(&ctx, &val1, &val2, &msg)
        },
    );
}

fn assert<M: Into<Dynamic>>(
    ctx: &NativeCallContext<'_>,
    condition: bool,
    get_msg: impl FnOnce() -> M,
) -> Result {
    match condition {
        true => Ok(()),
        false => Err(Box::new(EvalAltResult::ErrorRuntime(
            get_msg().into(),
            ctx.position(),
        ))),
    }
}

fn assert_eq(ctx: &NativeCallContext<'_>, val1: &Dynamic, val2: &Dynamic, msg: &Dynamic) -> Result {
    assert(
        &ctx,
        ctx.call_fn("==", (val1.clone(), val2.clone()))?,
        || {
            format!(
                "{}: {} == {}",
                rhai_to_string(ctx, msg),
                rhai_to_debug(ctx, val1),
                rhai_to_debug(ctx, val2),
            )
        },
    )
}

fn assert_neq(
    ctx: &NativeCallContext<'_>,
    val1: &Dynamic,
    val2: &Dynamic,
    msg: &Dynamic,
) -> Result {
    assert(
        &ctx,
        ctx.call_fn("!=", (val1.clone(), val2.clone()))?,
        || {
            format!(
                "{}: {} != {}",
                rhai_to_string(ctx, msg),
                rhai_to_debug(ctx, val1),
                rhai_to_debug(ctx, val2),
            )
        },
    )
}
