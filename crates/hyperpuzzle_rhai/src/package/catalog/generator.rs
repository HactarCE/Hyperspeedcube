use std::sync::Arc;

use eyre::{bail, eyre};
use hyperpuzzle_core::catalog::{BuildCtx, Generator};
use itertools::Itertools;
use rhai::Array;

use super::*;

pub(super) fn generator_from_rhai_map<T: 'static + Send + Sync>(
    ctx: &Ctx<'_>,
    eval_tx: RhaiEvalRequestTx,
    map: Map,
    generate_from_spec: impl 'static + Send + Sync + Fn(&Ctx<'_>, BuildCtx, Map) -> Result<T>,
) -> Result<Generator<T>> {
    let_from_map!(ctx, map, {
        let id: String;
        let name: Option<String>;
        let params: Array;
        let r#gen: FnPtr;
    });

    hyperpuzzle_core::validate_id(&id).eyrefmt()?;

    let name = match name {
        Some(name) => name,
        None => {
            let warn_msg = format!("missing `name` for color system generator `{id}`");
            warn(ctx, warn_msg);
            id.clone()
        }
    };

    let params: Vec<GeneratorParam> = params
        .into_iter()
        .map(|p| param_from_rhai(ctx, p))
        .try_collect()?;

    let generate_from_spec = Arc::new(generate_from_spec);
    let generate_fn_ptr = r#gen.clone();

    let id_clone = id.clone();
    let name_clone = name.clone();
    let params_clone = params.clone();

    let generate_fn = move |ctx: Ctx<'_>, (build_ctx, args): (BuildCtx, Vec<String>)| {
        let args = args.iter().map(|s| s.to_string()).collect_vec();

        let expected_len = params.len();
        let actual_len = args.len();
        if expected_len != actual_len {
            eyre::bail!("expected {expected_len} params; got {actual_len}");
        }

        let mut this = Dynamic::from(Map::from_iter([
            ("id".into(), id.clone().into()),
            ("name".into(), name.clone().into()),
        ]))
        .into_read_only();

        let param_values: Array = std::iter::zip(&params, &args)
            .map(|(param, arg)| param.value_from_str(arg))
            .map_ok(|param_value| param_value_into_rhai(&ctx, &param_value))
            .try_collect()?;

        let return_value = generate_fn_ptr
            .call_raw(&ctx, Some(&mut this), [Dynamic::from(param_values)])
            .map_err(|e| eyre!("error running `gen` function: {e}"))?;

        if return_value.is_string() {
            let string: String = from_rhai(&ctx, return_value).map_err(|e| eyre!("{e}"))?;
            Ok(Redirectable::Redirect(string))
        } else if return_value.is_map() {
            let mut map: Map = from_rhai(&ctx, return_value).map_err(|e| eyre!("{e}"))?;

            let id = hyperpuzzle_core::generated_id(&id, &args);
            if map.insert("id".into(), id.into()).is_some() {
                bail!("generated object must not have `id` specified");
            }

            Ok(Redirectable::Direct(Arc::new(generate_from_spec(
                &ctx, build_ctx, map,
            )?)))
        } else {
            let e = ConvertError::new_expected_str(&ctx, "string or map", Some(&return_value));
            Err(eyre!("{e}"))
        }
    };

    let generate = crate::util::rhai_eval_fn(ctx, eval_tx, &r#gen, generate_fn);

    Ok(Generator {
        id: id_clone,
        name: name_clone,

        params: params_clone,
        generate: Box::new(move |build_ctx, args| generate((build_ctx, args)).and_then(|r| r)),
    })
}

fn param_from_rhai(ctx: &Ctx<'_>, value: Dynamic) -> Result<GeneratorParam> {
    let_from_map!(ctx, from_rhai(ctx, value)?, {
        let name: String;
        let r#type: String;
        let init: Dynamic;
        let min: Option<i64>;
        let max: Option<i64>;
    });

    let ty = match r#type.as_str() {
        "int" => {
            let min = min.ok_or("`int` type requires `min`")?;
            let max = max.ok_or("`int` type requires `max`")?;
            GeneratorParamType::Int { min, max }
        }
        s => return Err(format!("unknown parameter type {s:?}").into()),
    };

    let default = param_value_from_rhai(ctx, &ty, &name, init)?;

    Ok(GeneratorParam { name, ty, default })
}

/// Converts a parameter value into a Lua value.
pub fn param_value_into_rhai(_ctx: &Ctx<'_>, value: &GeneratorParamValue) -> Dynamic {
    match value {
        GeneratorParamValue::Int(i) => (*i as rhai::INT).into(),
    }
}

/// Converts a Lua value to a value for this parameter and returns an error if
/// it is invalid.
pub fn param_value_from_rhai(
    ctx: &Ctx<'_>,
    ty: &GeneratorParamType,
    name: &str,
    value: Dynamic,
) -> Result<GeneratorParamValue> {
    match ty {
        GeneratorParamType::Int { min, max } => {
            let i = from_rhai(ctx, value)?;
            if i > *max {
                Err(format!("value {i:?} for parameter {name:?} is greater than {max}").into())
            } else if i < *min {
                Err(format!("value {i:?} for parameter {name:?} is less than {min}").into())
            } else {
                Ok(GeneratorParamValue::Int(i))
            }
        }
    }
}
