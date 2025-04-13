use super::*;

pub fn init_engine(engine: &mut Engine) {
    engine
        .register_custom_syntax(["with", "$expr$", "$block$"], false, |ctx, exprs| {
            let state_mutex = RhaiState::get(&mut *ctx);
            let mut state = state_mutex.lock();
            if state.symmetry.is_some() {
                return Err("nesting symmetry blocks is not allowed".into());
            }
            let symmetry = ctx.eval_expression_tree(&exprs[0])?;
            state.symmetry = Some(from_rhai(&mut *ctx, symmetry)?);
            drop(state);
            let result = ctx.eval_expression_tree(&exprs[1]);
            state_mutex.lock().symmetry = None;
            result
        })
        .expect("error registering custom syntax");
}
