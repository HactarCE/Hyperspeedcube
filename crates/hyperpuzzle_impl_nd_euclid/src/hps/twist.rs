use std::fmt;

use hypermath::pga::Motor;
use hyperpuzzle_core::{Multiplier, NameSpec, Twist};
use hyperpuzzlescript::{
    Builtins, ErrorExt, FnValue, Map, Result, Span, Spanned, Value, ValueData, hps_fns,
    impl_simple_custom_type,
};

use super::{HpsAxis, HpsTwistSystem};

#[derive(Clone, PartialEq, Eq)]
pub struct HpsTwist {
    pub id: Twist,
    pub multiplier: Multiplier,
    pub twists: HpsTwistSystem,
}
impl_simple_custom_type!(HpsTwist = "euclid.Twist", field_get = Self::impl_field_get);
impl HpsTwist {
    fn impl_field_get(
        &self,
        span: Span,
        (field, _field_span): Spanned<&str>,
    ) -> Result<Option<ValueData>> {
        Ok(match field {
            "id" => Some((self.id.0 as u64).into()),
            "axis" => Some(self.axis().at(span)?.into()),
            "transform" => Some(self.transform().at(span)?.into()),
            "name" => Some(self.name().at(span)?.map(|name| name.preferred).into()),
            _ => None,
        })
    }

    pub fn axis(&self) -> eyre::Result<HpsAxis> {
        Ok(HpsAxis {
            id: self.twists.twist_axis(self.id)?,
            axes: self.twists.axes(),
        })
    }
    pub fn transform(&self) -> eyre::Result<Motor> {
        self.twists.twist_transform(self.id)
    }
    pub fn name(&self) -> eyre::Result<Option<NameSpec>> {
        self.twists.twist_name(self.id)
    }
}
impl fmt::Debug for HpsTwist {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "twist {}", self.id)
    }
}
impl fmt::Display for HpsTwist {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        super::fmt_puzzle_element(f, "twists", self.name().unwrap_or(None), self.id)
    }
}

/// Adds the built-ins.
pub fn define_in(builtins: &mut Builtins<'_>) -> Result<()> {
    builtins.set_custom_ty::<HpsTwist>()?;

    builtins.set_fns(hps_fns![
        fn rev(ctx: EvalCtx, twist: HpsTwist) -> HpsTwist {
            HpsTwist {
                id: twist.id,
                multiplier: twist.multiplier.inv().at(ctx.caller_span)?,
                twists: twist.twists,
            }
        }

        fn transform(ctx: EvalCtx, (twist, twist_span): HpsTwist, object: Value) -> Value {
            let fn_value = ctx.scope.get("transform").unwrap_or_default();
            let transform = ValueData::from(twist.transform().at(ctx.caller_span)?).at(twist_span);
            let args = vec![transform, object];
            fn_value
                .as_ref::<FnValue>()?
                .call(fn_value.span, ctx, args, Map::new())?
        }
    ])
}
