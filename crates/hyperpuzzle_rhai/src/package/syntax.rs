use super::{
    catalog::{RhaiAxisSystem, RhaiTwistSystem},
    *,
};

pub fn init_engine(engine: &mut Engine) {
    engine
        .register_custom_syntax(["with", "$expr$", "$block$"], false, |mut ctx, exprs| {
            let symmetry = ctx.eval_expression_tree(&exprs[0])?;
            let symmetry = from_rhai(&mut ctx, symmetry)?;
            RhaiState::with_symmetry(ctx, symmetry, |ctx| ctx.eval_expression_tree(&exprs[1]))
        })
        .expect("error registering custom syntax");

    engine
        .register_custom_syntax(
            ["use", "$expr$", "from", "$expr$"],
            true,
            |mut ctx, exprs| {
                // IIFE to mimic try_block
                let keys: Vec<&ImmutableString> = (|| match exprs[0].as_ref() {
                    rhai::Expr::Array(thin_vec, _position) => thin_vec
                        .iter()
                        .map(|expr| match expr {
                            rhai::Expr::Variable(var, _index, _position) => Ok(&var.1),
                            _ => Err(()),
                        })
                        .collect(),
                    _ => Err(()),
                })()
                .map_err(|()| "expected array of identifiers after 'use'")?;

                let source = ctx.eval_expression_tree(&exprs[1])?;
                let map = from_rhai::<RhaiMapType>(&mut ctx, source)?;
                for key in keys {
                    match map.get(key.as_str()) {
                        Some(value) => ctx.scope_mut().set_or_push(key, value),
                        None => return Err(format!("missing key {key:?}").into()),
                    };
                }
                Ok(Dynamic::UNIT)
            },
        )
        .expect("error registering custom syntax");
}

enum RhaiMapType {
    Map(Map),
    AxisSystem(RhaiAxisSystem),
    TwistSystem(RhaiTwistSystem),
}
impl FromRhai for RhaiMapType {
    fn expected_string() -> String {
        "map, axis system, or twist system".to_owned()
    }

    fn try_from_rhai(ctx: impl RhaiCtx, value: Dynamic) -> Result<Self, ConvertError> {
        Err(value)
            .or_else(|value| value.try_cast_result().map(Self::Map))
            .or_else(|value| value.try_cast_result().map(Self::AxisSystem))
            .or_else(|value| value.try_cast_result().map(Self::TwistSystem))
            .map_err(|v| ConvertError::new::<Self>(ctx, Some(&v)))
    }
}
impl RhaiMapType {
    fn get(&self, key: &str) -> Option<Dynamic> {
        match self {
            RhaiMapType::Map(map) => map.get(key).cloned(),
            RhaiMapType::AxisSystem(axes) => axes.get(key).unwrap_or(None).map(Dynamic::from),
            RhaiMapType::TwistSystem(twists) => twists.get(key).unwrap_or(None).map(Dynamic::from),
        }
    }
}
