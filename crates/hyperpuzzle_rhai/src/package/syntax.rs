use super::*;

pub fn init_engine(engine: &mut Engine) {
    engine
        .register_custom_syntax(["with", "$expr$", "$block$"], false, |mut ctx, exprs| {
            let symmetry = ctx.eval_expression_tree(&exprs[0])?;
            let symmetry = from_rhai(&mut ctx, symmetry)?;
            RhaiState::with_symmetry(ctx, symmetry, |ctx| ctx.eval_expression_tree(&exprs[1]))
        })
        .expect("error registering custom syntax");
}
