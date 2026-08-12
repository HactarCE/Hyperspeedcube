//! Utility functions for defining generators for catalog entries.
//!
//! This module does not actually define any HPS API.

use std::sync::Arc;

use hyperpuzzle_core::{GeneratorParam, GeneratorParamType, TypedCatalogIdValue};
use itertools::Itertools;

use crate::engine::{HpsGenerator, HpsGeneratorFn};
use crate::util::pop_map_key;
use crate::{ErrorExt, FnValue, Map, Result, Spanned, Type, Value};

pub(super) fn hps_generator_from_kwargs(mut kwargs: Map) -> Result<HpsGenerator> {
    pop_kwarg!(kwargs, (id, id_span): String);
    let id = id.parse().at(id_span)?;

    let name = kwargs
        .get("name")
        .map(|v| Ok(v.as_ref::<str>()?.to_owned()))
        .transpose()?;

    let gen_fn = if kwargs.contains_key("gen") {
        kwargs.swap_remove("name");
        pop_kwarg!(kwargs, params: Vec<Spanned<Arc<Map>>>);
        pop_kwarg!(kwargs, (r#gen, gen_fn_span): Arc<FnValue>);
        Some(HpsGeneratorFn {
            params: params_from_array(params)?,
            subset_param: None, // TODO
            gen_fn: r#gen,
            gen_fn_span,
        })
    } else {
        None
    };

    Ok(HpsGenerator {
        id,
        name,
        kwargs,
        gen_fn,
    })
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
    let default = param_value_from_hps(&ty, &name, pop_map_key(&mut map, map_span, "default")?)?
        .into_untyped();
    Ok(GeneratorParam { name, ty, default })
}

fn param_value_from_hps(
    ty: &GeneratorParamType,
    name: &str,
    value: Value,
) -> Result<TypedCatalogIdValue> {
    let span = value.span;
    match ty {
        GeneratorParamType::Bool => Ok(TypedCatalogIdValue::Bool(value.to()?)),
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
            Ok(TypedCatalogIdValue::Int(i))
        }
        GeneratorParamType::Puzzle { .. } | GeneratorParamType::Id { .. } => Ok(
            TypedCatalogIdValue::Id(value.to::<String>()?.parse().at(span)?),
        ),
        GeneratorParamType::List(inner) => Ok(TypedCatalogIdValue::List(
            value
                .to::<Vec<_>>()?
                .into_iter()
                .map(|e| param_value_from_hps(inner, &format!("list element of {name:?}"), e))
                .try_collect()?,
        )),
    }
}
