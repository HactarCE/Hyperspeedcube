use super::*;

fn assert_impl<M: Into<Dynamic>>(
    ctx: &NativeCallContext<'_>,
    condition: bool,
    get_msg: impl FnOnce() -> M,
) -> Result<()> {
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
    val1: &Dynamic,
    val2: &Dynamic,
    msg: &Dynamic,
) -> Result<()> {
    assert_impl(
        &ctx,
        ctx.call_fn("==", (val1.clone(), val2.clone()))?,
        || {
            format!(
                "{}: {} == {}",
                to_string(ctx, msg),
                to_debug(ctx, val1),
                to_debug(ctx, val2),
            )
        },
    )
}

fn assert_neq_impl(
    ctx: &NativeCallContext<'_>,
    val1: &Dynamic,
    val2: &Dynamic,
    msg: &Dynamic,
) -> Result<()> {
    assert_impl(
        &ctx,
        ctx.call_fn("!=", (val1.clone(), val2.clone()))?,
        || {
            format!(
                "{}: {} != {}",
                to_string(ctx, msg),
                to_debug(ctx, val1),
                to_debug(ctx, val2),
            )
        },
    )
}

fn to_string(ctx: &NativeCallContext<'_>, val: &Dynamic) -> String {
    ctx.call_fn::<String>(rhai::FUNC_TO_STRING, (val.clone(),))
        .unwrap_or_else(|_| val.to_string())
}

fn to_debug(ctx: &NativeCallContext<'_>, val: &Dynamic) -> String {
    ctx.call_fn::<String>(rhai::FUNC_TO_DEBUG, (val.clone(),))
        .unwrap_or_else(|_| val.to_string())
}

#[export_module]
pub(super) mod rhai_mod {
    // assert(cond)
    #[rhai_fn(global, return_raw)]
    pub fn assert(ctx: NativeCallContext<'_>, condition: bool) -> Result<()> {
        assert_impl(&ctx, condition, || "assertion error")
    }

    // assert(cond, msg)
    #[rhai_fn(global, return_raw, name = "assert")]
    pub fn assert_with_msg(
        ctx: NativeCallContext<'_>,
        condition: bool,
        msg: Dynamic,
    ) -> Result<()> {
        assert_impl(&ctx, condition, || msg)
    }

    // assert_eq(left, right)
    #[rhai_fn(global, return_raw)]
    pub fn assert_eq(ctx: NativeCallContext<'_>, val1: Dynamic, val2: Dynamic) -> Result<()> {
        assert_eq_impl(&ctx, &val1, &val2, &"assertion failed".into())
    }

    // assert_eq(left, right, msg)
    #[rhai_fn(global, return_raw, name = "assert_eq")]
    pub fn assert_eq_with_msg(
        ctx: NativeCallContext<'_>,
        val1: Dynamic,
        val2: Dynamic,
        msg: Dynamic,
    ) -> Result<()> {
        assert_eq_impl(&ctx, &val1, &val2, &msg)
    }

    // assert_neq(left, right)
    #[rhai_fn(global, return_raw)]
    pub fn assert_neq(ctx: NativeCallContext<'_>, val1: Dynamic, val2: Dynamic) -> Result<()> {
        assert_neq_impl(&ctx, &val1, &val2, &"assertion failed".into())
    }

    // assert_neq(left, right, msg)
    #[rhai_fn(global, return_raw, name = "assert_neq")]
    pub fn assert_neq_with_msg(
        ctx: NativeCallContext<'_>,
        val1: Dynamic,
        val2: Dynamic,
        msg: Dynamic,
    ) -> Result<()> {
        assert_neq_impl(&ctx, &val1, &val2, &msg)
    }
}
