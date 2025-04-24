use std::sync::Arc;

use parking_lot::Mutex;

use super::*;

const GET_GLOBAL_STATE_FN: &str = "__internals__get_global_state";

pub fn init_engine(engine: &mut Engine) {
    let state = Arc::new(Mutex::new(RhaiState::default()));
    engine.register_fn(GET_GLOBAL_STATE_FN, move || Arc::clone(&state));
}

/// Global state.
#[derive(Debug, Default)]
pub(super) struct RhaiState {
    pub ndim: Option<u8>,
    pub symmetry: Option<types::symmetry::RhaiSymmetry>,
}
impl RhaiState {
    /// Returns the global state.
    ///
    /// # Panics
    ///
    /// Panics if the appropriate [`init_engine()`] has not been called on the
    /// given Rhai engine.
    pub fn get(mut ctx: impl RhaiCtx) -> Arc<Mutex<RhaiState>> {
        ctx.call_rhai_native_fn(GET_GLOBAL_STATE_FN, vec![])
            .expect("error getting global Rhai state")
    }

    /// Returns the current number of dimensions.
    pub fn get_ndim(ctx: impl RhaiCtx) -> Result<u8> {
        (Self::get(ctx).lock().ndim)
            .ok_or_else(|| "not in a space with a number of dimensions".into())
    }

    /// Executes code with the given number of dimensions.
    pub fn with_ndim<C: RhaiCtx, R>(
        mut ctx: C,
        ndim: u8,
        f: impl FnOnce(C) -> Result<R>,
    ) -> Result<R> {
        if !(1..=hypermath::MAX_NDIM).contains(&ndim) {
            return Err(format!("invalid number of dimensions: {ndim}").into());
        }
        let state = Self::get(&mut ctx);
        let old_ndim = state.lock().ndim.replace(ndim);
        let result = f(ctx);
        state.lock().ndim = old_ndim;
        result
    }

    /// Executes code with the given symmetry.
    pub fn with_symmetry<C: RhaiCtx, R>(
        mut ctx: C,
        symmetry: types::symmetry::RhaiSymmetry,
        f: impl FnOnce(C) -> Result<R>,
    ) -> Result<R> {
        let state_mutex = Self::get(&mut ctx);
        let mut state = state_mutex.lock();
        if state.symmetry.is_some() {
            return Err("nesting symmetry blocks is not allowed".into());
        }
        state.symmetry = Some(symmetry);
        drop(state);
        let result = f(ctx);
        state_mutex.lock().symmetry = None;
        result
    }
}
