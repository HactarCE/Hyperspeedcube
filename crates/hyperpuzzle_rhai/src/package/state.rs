use parking_lot::Mutex;
use std::sync::Arc;

use super::*;

const GET_GLOBAL_STATE_FN: &str = "__internals__get_global_state";

pub fn init_engine(engine: &mut Engine) {
    let state = Arc::new(Mutex::new(RhaiState::default()));
    engine.register_fn(GET_GLOBAL_STATE_FN, move || Arc::clone(&state));
}

/// Global state.
#[derive(Debug, Default)]
pub(super) struct RhaiState {
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
        ctx.call_rhai_native_fn(GET_GLOBAL_STATE_FN, ())
            .expect("error getting global Rhai state")
    }
}
