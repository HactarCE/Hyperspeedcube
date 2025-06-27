use std::{str::FromStr, sync::Arc};

use ecow::eco_format;
use eyre::eyre;
use hyperpuzzle_core::{
    Catalog, ColorSystem, ColorSystemGenerator, DefaultColor, NameSpecBiMapBuilder, PerColor,
    catalog::BuildTask,
};
use indexmap::IndexMap;

use crate::{
    Error, ErrorExt, EvalCtx, EvalRequestTx, FnValue, List, Map, Result, Scope, Spanned, Str,
    util::pop_map_key,
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

            let meta = super::generators::GeneratorMeta {
                id,
                params: super::generators::params_from_array(params)?,
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

                        // IIFE to mimic try_block
                        (|| {
                            meta.generate_spec(&mut ctx, param_values)?.try_map(|spec| {
                                color_system_from_kwargs(&mut ctx, spec).map(Arc::new)
                            })
                        })()
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
