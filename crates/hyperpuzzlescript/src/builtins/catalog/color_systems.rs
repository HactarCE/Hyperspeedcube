use std::{str::FromStr, sync::Arc};

use ecow::eco_format;
use eyre::eyre;
use hyperpuzzle_core::{
    Catalog, ColorSystem, ColorSystemGenerator, DefaultColor, GeneratorParam, GeneratorParamType,
    GeneratorParamValue, NameSpecBiMapBuilder, PerColor, Redirectable, catalog::BuildTask,
};
use indexmap::IndexMap;
use itertools::Itertools;

use crate::{
    Error, ErrorExt, EvalCtx, EvalRequestTx, FnValue, List, Map, Num, Result, Scope, Span, Spanned,
    Str, Type, Value, ValueData, util::pop_map_key,
};

/// Adds the built-in functions to the scope.
pub fn define_in(scope: &Scope, catalog: &Catalog, eval_tx: &EvalRequestTx) -> Result<()> {
    let cat = catalog.clone();
    scope.register_builtin_functions(hps_fns![
        /// Adds a color system to the catalog.
        #[kwargs(kwargs)]
        fn add_color_system(ctx: EvalCtx) -> () {
            cat.add_color_system(Arc::new(color_system_from_kwargs(ctx, kwargs)?))
                .at(ctx.caller_span)?;
        }
    ])?;

    let cat = catalog.clone();
    let tx = eval_tx.clone();
    scope.register_builtin_functions(hps_fns![
        /// Adds a color system generator to the catalog.
        #[kwargs(
            id: String,
            name: String = {
                ctx.warn(eco_format!("missing `name` for color system generator `{id}`"));
                id.clone()
            },
            params: Vec<Spanned<Arc<Map>>>,
            (r#gen, gen_span): Arc<FnValue>,
        )]
        fn add_color_system_generator(ctx: EvalCtx) -> () {
            let caller_span = ctx.caller_span;

            let tx = tx.clone();

            let meta = GeneratorMeta {
                id,
                params: params
                    .into_iter()
                    .map(generator_param_from_map)
                    .try_collect()?,
                gen_fn: r#gen,
                gen_span,
            };

            let generator = ColorSystemGenerator {
                id: meta.id.clone(),
                name,
                params: meta.params.clone(),
                generate: Box::new(move |build_ctx, param_values| {
                    build_ctx.progress.lock().task = BuildTask::GeneratingSpec;

                    let scope = Scope::new();
                    let meta = meta.clone();

                    tx.eval_blocking(move |runtime| {
                        let mut ctx = EvalCtx {
                            scope: &scope,
                            runtime,
                            caller_span,
                            exports: &mut None,
                        };

                        meta.generate_color_system(&mut ctx, param_values)
                            .map_err(|e| {
                                let s = e.to_string(&*ctx.runtime);
                                ctx.runtime.report_diagnostic(e);
                                eyre!(s)
                            })
                    })
                }),
            };

            cat.add_color_system_generator(Arc::new(generator))
                .at(ctx.caller_span)?;
        }
    ])?;

    Ok(())
}

#[derive(Debug, Clone)]
struct GeneratorMeta {
    id: String,
    params: Vec<GeneratorParam>,
    gen_fn: Arc<FnValue>,
    gen_span: Span,
}

impl GeneratorMeta {
    fn generate_color_system(
        &self,
        ctx: &mut EvalCtx<'_>,
        generator_param_values: Vec<String>,
    ) -> Result<Redirectable<Arc<ColorSystem>>> {
        let expected = self.params.len();
        let got = generator_param_values.len();
        if expected != got {
            let generator_id = &self.id;
            return Err(
                format!("generator {generator_id} expects {expected} params; got {got}")
                    .at(crate::BUILTIN_SPAN),
            );
        }

        let params = std::iter::zip(&self.params, &generator_param_values)
            .map(|(p, s)| {
                let v = &p.value_from_str(s).at(ctx.caller_span)?;
                Ok(param_value_into_hps(v))
            })
            .try_collect()?;

        let user_gen_fn_output = self.gen_fn.call(self.gen_span, ctx, params, Map::new())?;

        match user_gen_fn_output.data {
            ValueData::Str(redirect_id) => Ok(Redirectable::Redirect(redirect_id.into())),
            ValueData::List(l) => {
                let mut iter = Arc::unwrap_or_clone(l).into_iter();
                let redirect_id = iter
                    .next()
                    .ok_or("empty redirect sequence".at(user_gen_fn_output.span))?
                    .to::<String>()?;
                let redirect_params: Vec<Str> = iter.map(|v| v.to()).try_collect()?;
                Ok(Redirectable::Redirect(if redirect_params.is_empty() {
                    redirect_id
                } else {
                    hyperpuzzle_core::generated_id(&redirect_id, redirect_params)
                }))
            }
            ValueData::Map(m) => {
                let mut params = Arc::unwrap_or_clone(m);
                let id_str = hyperpuzzle_core::generated_id(&self.id, &generator_param_values);
                let id = ValueData::Str(id_str.into()).at(crate::BUILTIN_SPAN);
                if let Some(old_id) = params.insert("id".into(), id) {
                    ctx.warn_at(old_id.span, "overwriting `id` from color system generator");
                }
                Ok(Redirectable::Direct(Arc::new(color_system_from_kwargs(
                    ctx, params,
                )?)))
            }
            _ => Err("return value of `gen` function must be string \
                      (ID redirect), list (ID redirect to generator), \
                      or map (color system specification)"
                .at(ctx.caller_span)),
        }
    }
}

fn generator_param_from_map((map, map_span): Spanned<Arc<Map>>) -> Result<GeneratorParam> {
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
    }
}

fn param_value_into_hps(value: &GeneratorParamValue) -> Value {
    match value {
        GeneratorParamValue::Int(i) => ValueData::Num(*i as Num),
    }
    .at(crate::BUILTIN_SPAN)
}

fn color_system_from_kwargs(ctx: &mut EvalCtx<'_>, kwargs: Map) -> Result<ColorSystem> {
    unpack_kwargs!(
        kwargs,
        id: String,
        name: String = {
            ctx.warn(eco_format!("missing `name` for color system `{id}`"));
            id.clone()
        },
        colors: Vec<Spanned<Arc<Map>>>,
        schemes: Option<Vec<Spanned<List>>>,
        default: Option<String>,
    );

    let mut names = NameSpecBiMapBuilder::new();
    let mut display_names = PerColor::new();

    let mut any_color_has_default = false;
    let mut default_scheme = PerColor::new();

    // Add colors.
    for (map, map_span) in colors {
        let mut map = Arc::unwrap_or_clone(map);

        let id = display_names.next_idx().at(ctx.caller_span)?;

        let (name_spec, name_span): Spanned<String> = pop_map_key(&mut map, map_span, "name")?;
        names.set(id, Some(name_spec.clone())).at(name_span)?;

        let display = pop_map_key::<Option<_>>(&mut map, map_span, "display")?
            .unwrap_or_else(|| hyperpuzzle_core::preferred_name_from_name_spec(&name_spec));
        display_names.push(display).at(map_span)?;

        let default_color =
            match pop_map_key::<Option<Spanned<Str>>>(&mut map, map_span, "default")? {
                None => DefaultColor::Unknown,
                Some((s, span)) => {
                    any_color_has_default = true;
                    DefaultColor::from_str(&s).at(span)?
                }
            };
        default_scheme.push(default_color).at(ctx.caller_span)?;
    }

    let names = names
        .build(display_names.len())
        .ok_or_else(|| Error::User("missing color name".into()).at(ctx.caller_span))?;

    // Add color schemes.
    let mut ret_schemes = IndexMap::new();
    if let Some(color_schemes_list) = schemes {
        if any_color_has_default {
            ctx.warn("per-color `default` is ignored when used with `schemes`");
        }

        for (mut map, map_span) in color_schemes_list {
            if map.len() != 2 {
                return Err(Error::User("expected list with 2 elements".into()).at(map_span));
            }
            let scheme_name = std::mem::take(&mut map[0]).to::<String>()?;
            let mut scheme_values = PerColor::<DefaultColor>::new_with_len(display_names.len());
            for (k, v) in map[1].as_ref::<Map>()? {
                let i = names
                    .id_from_name(k)
                    .ok_or_else(|| format!("no color with name {k:?}"))
                    .at(map[1].span)?;
                scheme_values[i] = v.as_ref::<str>()?.parse().at(v.span)?;
            }
            ret_schemes.insert(scheme_name, scheme_values);
        }
    } else {
        ret_schemes.insert(
            hyperpuzzle_core::DEFAULT_COLOR_SCHEME_NAME.to_owned(),
            default_scheme,
        );
    }

    let default_scheme =
        default.unwrap_or_else(|| hyperpuzzle_core::DEFAULT_COLOR_SCHEME_NAME.to_owned());
    if !ret_schemes.contains_key(&default_scheme) {
        ctx.warn(format!(
            "default color scheme {default_scheme:?} does not exist"
        ));
    }

    Ok(ColorSystem {
        id,
        name,
        names,
        display_names,
        schemes: ret_schemes,
        default_scheme,
        orbits: vec![],
    })
}
