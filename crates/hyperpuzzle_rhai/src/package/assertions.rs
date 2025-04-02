use super::*;

#[export_module]
pub(super) mod rhai_mod {
    // assert(cond)
    #[rhai_fn(global, return_raw)]
    pub fn assert(ctx: NativeCallContext<'_>, condition: bool) -> RhaiFnOutput {
        assert_impl(&ctx, condition, || "assertion error")
    }

    // assert(cond, msg)
    #[rhai_fn(global, return_raw, name = "assert")]
    pub fn assert_with_msg(
        ctx: NativeCallContext<'_>,
        condition: bool,
        msg: Dynamic,
    ) -> RhaiFnOutput {
        assert_impl(&ctx, condition, || msg)
    }

    // assert_eq(left, right)
    #[rhai_fn(global, return_raw)]
    pub fn assert_eq(ctx: NativeCallContext<'_>, val1: Dynamic, val2: Dynamic) -> RhaiFnOutput {
        assert_eq_impl(&ctx, val1, val2, "assertion failed")
    }

    // assert_eq(left, right, msg)
    #[rhai_fn(global, return_raw, name = "assert_eq")]
    pub fn assert_eq_with_msg(
        ctx: NativeCallContext<'_>,
        val1: Dynamic,
        val2: Dynamic,
        msg: Dynamic,
    ) -> RhaiFnOutput {
        assert_eq_impl(&ctx, val1, val2, msg)
    }
}

fn assert_impl<M: Into<Dynamic>>(
    ctx: &NativeCallContext<'_>,
    condition: bool,
    get_msg: impl FnOnce() -> M,
) -> Result<(), Box<EvalAltResult>> {
    match condition {
        true => Ok(()),
        false => Err(Box::new(EvalAltResult::ErrorRuntime(
            get_msg().into(),
            ctx.position(),
        ))),
    }
}

fn assert_eq_impl(
    ctx: &NativeCallContext<'_>,
    val1: Dynamic,
    val2: Dynamic,
    msg: impl fmt::Display,
) -> Result<(), Box<EvalAltResult>> {
    assert_impl(
        &ctx,
        ctx.call_fn("==", (val1.clone(), val2.clone()))?,
        || format!("{msg}: {val1} == {val2}"),
    )
}
