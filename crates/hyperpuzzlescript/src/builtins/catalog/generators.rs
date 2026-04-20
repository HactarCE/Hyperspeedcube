//! Utility functions for defining generators for catalog entries.
//!
//! This module does not actually define any HPS API.

use std::collections::HashMap;
use std::sync::Arc;

use ecow::{EcoString, eco_format};
use hyperpuzzle_core::{
    CatalogArgValue, CatalogId, GeneratorParam, GeneratorParamType, GeneratorParamValue,
    Redirectable,
};
use itertools::Itertools;

use crate::util::pop_map_key;
use crate::{
    ErrorExt, EvalCtx, EvalRequestTx, FnValue, Map, Num, Result, Runtime, Scope, Span, Spanned,
    Type, Value, ValueData,
};

#[derive(Debug, Clone)]
pub(super) struct HpsGenerator {
    pub def_span: Span,
    pub id: CatalogId,
    pub id_span: Span,
    pub params: Vec<GeneratorParam>,
    pub params_span: Span,
    pub gen_fn: Arc<FnValue>,
    pub gen_span: Span,
    pub extra: Arc<Map>,
}
impl HpsGenerator {
    pub(super) fn generated_id(&self, param_values: Vec<CatalogArgValue>) -> Result<EcoString> {
        let id = CatalogId::new(&*self.id.base, param_values.clone())
            .ok_or_else(|| "invalid generator ID".at(self.id_span))?;
        Ok(eco_format!("{id}"))
    }

    /// Creates a [`Map`] for a generated object, then evaluates `map_fn` on
    /// them with an HPS evaluation context and returns the result.
    ///
    /// If there exists a matching example from `examples_by_id`, then that is
    /// returned instead.
    ///
    /// In the case of an ID redirect, `map_fn` is not called.
    ///
    /// **Note: This method blocks waiting on a result from the HPS thread.**
    pub(super) fn generate_on_hps_thread_with_examples<T: 'static + Send + Sync>(
        &self,
        tx: &EvalRequestTx,
        param_values: Vec<CatalogArgValue>,
        examples_by_id: &HashMap<String, Arc<T>>,
        map_fn: impl 'static + Send + Sync + FnOnce(&mut EvalCtx<'_>, Map) -> Result<Arc<T>>,
    ) -> eyre::Result<Redirectable<Arc<T>>> {
        if let Ok(id) = self.generated_id(param_values.clone())
            && let Some(example_puzzle) = examples_by_id.get(&*id)
        {
            Ok(Redirectable::Direct(Arc::clone(example_puzzle)))
        } else {
            self.generate_on_hps_thread(tx, param_values, map_fn)
        }
    }

    /// Creates a [`Map`] for a generated object, then evaluates `map_fn` on
    /// them with an HPS evaluation context and returns the result.
    ///
    /// In the case of an ID redirect, `map_fn` is not called.
    ///
    /// **Note: This method blocks waiting on a result from the HPS thread.**
    pub(super) fn generate_on_hps_thread<T: 'static + Send>(
        &self,
        tx: &EvalRequestTx,
        param_values: Vec<CatalogArgValue>,
        map_fn: impl 'static + Send + Sync + FnOnce(&mut EvalCtx<'_>, Map) -> Result<T>,
    ) -> eyre::Result<Redirectable<T>> {
        let this = self.clone();
        tx.eval_blocking(move |runtime| this.generate(runtime, param_values, map_fn))
    }

    /// Creates a [`Map`] for a generated object, then evaluates `map_fn` on
    /// them with an HPS evaluation context and returns the result.
    ///
    /// In the case of an ID redirect, `map_fn` is not called.
    pub(super) fn generate<T: 'static + Send>(
        &self,
        runtime: &mut Runtime,
        param_values: Vec<CatalogArgValue>,
        map_fn: impl 'static + Send + Sync + FnOnce(&mut EvalCtx<'_>, Map) -> Result<T>,
    ) -> eyre::Result<Redirectable<T>> {
        // IIFE to mimic try_block
        let result = (|| {
            let mut scope = Scope::default();
            scope.special.id = Some(self.generated_id(param_values.clone())?);
            let scope = Arc::new(scope);

            let mut ctx = EvalCtx {
                scope: &scope,
                runtime,
                caller_span: self.def_span,
                exports: &mut None,
                stack_depth: 0,
            };

            let expected = self.params.len();
            let got = param_values.len();
            if expected != got {
                let generator_id = &self.id;
                return Err(format!(
                    "generator {generator_id} expects {expected} params; got {got}"
                )
                .at(self.params_span));
            }

            let params = std::iter::zip(&self.params, &param_values)
                .map(|(p, s)| {
                    let v = &p.value_from_arg(s).at(ctx.caller_span)?;
                    Ok(param_value_into_hps(v))
                })
                .try_collect()?;

            let user_gen_fn_output =
                self.gen_fn
                    .call(self.gen_span, &mut ctx, params, Map::new())?;

            match user_gen_fn_output.data {
                ValueData::Str(redirect_id) => Ok(Redirectable::Redirect(redirect_id.into())),
                ValueData::Map(m) => {
                    let mut map = Arc::unwrap_or_clone(m);
                    let id_str_value =
                        ValueData::Str(self.generated_id(param_values)?).at(self.id_span);
                    if let Some(old_id) = map.insert("id".into(), id_str_value) {
                        ctx.warn_at(old_id.span, "overwriting `id` from generator");
                    }
                    for (k, v) in &*self.extra {
                        if let Some(old_val) = map.insert(k.clone(), v.clone()) {
                            ctx.warn_at(old_val.span, format!("overwriting `{k}` from generator"));
                        }
                    }
                    Ok(Redirectable::Direct(map_fn(&mut ctx, map)?))
                }
                _ => Err("return value of `gen` function must be string (ID \
                          redirect), list (ID redirect to generator), or map"
                    .at(ctx.caller_span)),
            }
        })();
        runtime.report_and_convert_to_eyre(result)
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
