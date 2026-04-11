//! Utility functions for defining generators for catalog entries.
//!
//! This module does not actually define any HPS API.

use std::sync::Arc;

use hyperpuzzle_core::{
    CatalogArgValue, CatalogId, GeneratorParam, GeneratorParamType, GeneratorParamValue,
    Redirectable,
};
use itertools::Itertools;

use crate::util::pop_map_key;
use crate::{ErrorExt, EvalCtx, FnValue, Map, Num, Result, Span, Spanned, Type, Value, ValueData};

#[derive(Debug, Clone)]
pub(super) struct GeneratorMeta {
    pub id: String,
    pub id_span: Span,
    pub params: Vec<GeneratorParam>,
    pub params_span: Span,
    pub gen_fn: Arc<FnValue>,
    pub gen_span: Span,
    pub extra: Map,
}
impl GeneratorMeta {
    pub(super) fn generate_spec(
        &self,
        ctx: &mut EvalCtx<'_>,
        generator_param_values: Vec<CatalogArgValue>,
    ) -> Result<Redirectable<Map>> {
        let expected = self.params.len();
        let got = generator_param_values.len();
        if expected != got {
            let generator_id = &self.id;
            return Err(
                format!("generator {generator_id} expects {expected} params; got {got}")
                    .at(self.params_span),
            );
        }

        let params = std::iter::zip(&self.params, &generator_param_values)
            .map(|(p, s)| {
                let v = &p.value_from_arg(s).at(ctx.caller_span)?;
                Ok(param_value_into_hps(v))
            })
            .try_collect()?;

        let user_gen_fn_output = self.gen_fn.call(self.gen_span, ctx, params, Map::new())?;

        match user_gen_fn_output.data {
            ValueData::Str(redirect_id) => Ok(Redirectable::Redirect(redirect_id.into())),
            ValueData::Map(m) => {
                let mut params = Arc::unwrap_or_clone(m);
                let id = CatalogId::new(&*self.id, generator_param_values.clone())
                    .ok_or_else(|| "invalid generator ID".at(self.id_span))?;
                let id_str_value = ValueData::Str(id.to_string().into()).at(self.id_span);
                if let Some(old_id) = params.insert("id".into(), id_str_value) {
                    ctx.warn_at(old_id.span, "overwriting `id` from generator");
                }
                for (k, v) in &self.extra {
                    if let Some(old_val) = params.insert(k.clone(), v.clone()) {
                        ctx.warn_at(old_val.span, format!("overwriting `{k}` from generator"));
                    }
                }
                Ok(Redirectable::Direct(params))
            }
            _ => Err("return value of `gen` function must be string (ID
                      redirect), list (ID redirect to generator), or map"
                .at(ctx.caller_span)),
        }
    }
}

pub(super) fn param_value_into_hps(value: &GeneratorParamValue) -> Value {
    match value {
        GeneratorParamValue::Int(i) => ValueData::Num(*i as Num),
        GeneratorParamValue::PuzzleId(p) => ValueData::Str(p.to_string().into()),
    }
    .at(crate::BUILTIN_SPAN)
}

pub(super) fn params_from_array(array: Vec<Spanned<Arc<Map>>>) -> Result<Vec<GeneratorParam>> {
    array.into_iter().map(param_from_map).collect()
}

fn param_from_map((map, map_span): Spanned<Arc<Map>>) -> Result<GeneratorParam> {
    let mut map = Arc::unwrap_or_clone(map);
    let name: String = pop_map_key(&mut map, map_span, "name")?;
    let (ty_value, ty_span) = pop_map_key(&mut map, map_span, "type")?;
    let ty = match ty_value {
        Type::Int => GeneratorParamType::Int {
            min: pop_map_key(&mut map, map_span, "min")?,
            max: pop_map_key(&mut map, map_span, "max")?,
        },
        other => {
            let allowed_types = &[Type::Int];
            return Err(format!(
                "invalid type {other} for generator parameter; allowed types: {allowed_types:?}",
            )
            .at(ty_span));
        }
    };
    let default = param_value_from_hps(&ty, &name, pop_map_key(&mut map, map_span, "default")?)?;
    Ok(GeneratorParam { name, ty, default })
}

fn param_value_from_hps(
    ty: &GeneratorParamType,
    name: &str,
    value: Value,
) -> Result<GeneratorParamValue> {
    let span = value.span;
    match ty {
        &GeneratorParamType::Int { min, max } => {
            let i = value.to()?;
            if i > max {
                return Err(
                    format!("value {i:?} for parameter {name:?} is greater than {max}").at(span),
                );
            }
            if i < min {
                return Err(
                    format!("value {i:?} for parameter {name:?} is less than {min}").at(span),
                );
            }
            Ok(GeneratorParamValue::Int(i))
        }
        GeneratorParamType::Puzzle => Ok(GeneratorParamValue::PuzzleId(
            value.to::<String>()?.parse().at(span)?,
        )),
    }
}
