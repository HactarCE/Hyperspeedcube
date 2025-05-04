use std::fmt;
use std::sync::{Arc, mpsc};

use eyre::eyre;
use rhai::{Dynamic, Engine, FnPtr, FuncRegistration, Position};

use crate::loader::RhaiEvalRequestTx;
use crate::{Ctx, RhaiCtx};

/// Emits a warning.
pub fn warn(mut ctx: impl RhaiCtx, msg: impl fmt::Display) {
    let args = vec![format!("{msg:#}").into()];
    match ctx.call_rhai_native_fn::<()>("warn", args) {
        Ok(()) => (),
        Err(e) => {
            log::error!("error calling Rhai warning function: {e}");
            log::warn!("{e}");
        }
    }
}
/// Returns a function that emits a warning.
pub fn warnf<'a, T: fmt::Display>(ctx: &'a Ctx<'_>) -> impl 'a + Copy + Fn(T) {
    |msg| warn(&mut &*ctx, msg)
}

pub fn warn_on_error<'a, E: std::error::Error>(
    ctx: &'a Ctx<'_>,
) -> impl 'a + Copy + Fn(E) -> crate::Result<()> {
    move |e| {
        warn(ctx, e);
        Ok(())
    }
}

/// Shortcut for `FuncRegistration::new(name).in_global_namespace()`.
pub fn new_fn(name: &str) -> FuncRegistration {
    FuncRegistration::new(name).in_global_namespace()
}

/// Calls Rhai `to_string()` on `val` and returns the result.
pub fn rhai_to_string(mut ctx: impl RhaiCtx, val: &Dynamic) -> String {
    ctx.call_rhai_native_fn::<String>(rhai::FUNC_TO_STRING, vec![val.clone()])
        .unwrap_or_else(|_| val.to_string())
}

/// Calls Rhai `to_debug()` on `val` and returns the result.
pub fn rhai_to_debug(mut ctx: impl RhaiCtx, val: &Dynamic) -> String {
    ctx.call_rhai_native_fn::<String>(rhai::FUNC_TO_DEBUG, vec![val.clone()])
        .unwrap_or_else(|_| val.to_string())
}

/// Returns a closure that can be called to evaluate `inner` on the Rhai thread.
///
/// All captures of `inner` should be cheap to clone.
pub fn rhai_eval_fn<A: 'static + Send + Sync, R: 'static + Send + Sync>(
    ctx: &Ctx<'_>,
    eval_tx: RhaiEvalRequestTx,
    fn_ptr: &FnPtr,
    inner: impl 'static + Clone + Send + Sync + Fn(Ctx<'_>, A) -> R,
) -> impl 'static + Clone + Send + Sync + Fn(A) -> eyre::Result<R> {
    let global_runtime_state = Arc::new(ctx.global_runtime_state().clone());
    let fn_name = fn_ptr.fn_name().to_owned();
    let pos = ctx.position();

    move |args| {
        let global_runtime_state = global_runtime_state.clone();
        let fn_name = fn_name.clone();
        let inner = inner.clone();

        let (result_tx, result_rx) = mpsc::channel::<R>();
        let rhai_eval_request = Box::new(move |engine: &mut Engine| {
            let ctx = Ctx::from((
                &*engine,
                fn_name.as_str(),
                None,
                &*global_runtime_state,
                pos,
            ));
            if result_tx.send(inner(ctx, args)).is_err() {
                log::warn!("error sending eval result to calling thread");
            }
        });

        eval_tx
            .send(rhai_eval_request)
            .map_err(|_| eyre!("error sending eval request to Rhai thread"))?;

        result_rx
            .recv()
            .map_err(|mpsc::RecvError| eyre!("channel disconnected; Rhai thread may have panicked"))
    }
}
