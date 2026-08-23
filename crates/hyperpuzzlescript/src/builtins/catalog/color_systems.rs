use std::str::FromStr;
use std::sync::Arc;

use ecow::eco_format;
use eyre::{OptionExt, bail, eyre};
use hyperpuzzle_core::catalog::CatalogHook;
use hyperpuzzle_core::{
    BuildCtx, CatalogBuilder, Color, ColorSystem, ComponentList, Names, PaletteColor, PerColor,
};
use indexmap::{IndexMap, IndexSet};
use itertools::Itertools;

use crate::util::{expect_end_of_map, pop_map_key};
use crate::{
    Builtins, ErrorExt, EvalRequestTx, FnValue, List, Map, Result, Runtime, Scope, Span, Str, Type,
    Value, ValueData,
};

/// Adds the built-in functions.
pub fn define_in(
    builtins: &mut Builtins<'_>,
    catalog: &CatalogBuilder,
    eval_tx: &EvalRequestTx,
) -> Result<()> {
    let cat = catalog.clone();
    let tx = eval_tx.clone();
    builtins.set_fns(hps_fns![
        /// Adds a color system or color system generator to the catalog.
        ///
        /// ## Single color system
        ///
        /// When used to define a single color system, this function takes the
        /// following named arguments:
        ///
        /// - `id: Str` — ID for the color system (e.g., `"cube"`)
        /// - `name: Str?` — Name for the color system (e.g., `"Cube"`)
        /// - `colors: List[Map]` — List of colors
        /// - `schemes: List[List]?` — List of color schemes
        /// - `default: Str?` — Name of the default color scheme
        ///
        /// ## Color system generator
        ///
        /// When used to define a color system generator, this function takes
        /// the following named arguments:
        ///
        /// - `id: Str` — ID for the color system generator (e.g., `"ngon"`)
        /// - `params: List[Map]` — List of generator parameters
        /// - `gen: Fn(..) -> Map` — Generator function
        ///
        /// The map returned by `gen` must have the following keys:
        ///
        /// - `name: Str?` — Name for the color system (e.g., `"{5}"`)
        /// - `colors: List[Map]` — List of colors
        /// - `schemes: List[List]?` — List of color schemes
        /// - `default: Str?` — Name of the default color scheme
        #[kwargs(kwargs)]
        fn add_color_system(ctx: EvalCtx) -> () {
            let hps_gen = super::generators::hps_generator_from_kwargs(ctx, kwargs)?;
            let caller_span = ctx.caller_span;
            cat.add(hps_gen.make_generator(&tx, move |build_ctx, tx, kwargs| {
                Ok(tx.eval_blocking_raw(move |runtime| {
                    color_system_from_kwargs(build_ctx, caller_span, runtime, kwargs)
                })?)
            }))
            .at(caller_span)?;
        }
    ])?;

    let cat = catalog.clone();
    let tx = eval_tx.clone();
    builtins.set_fns(hps_fns![
        /// Adds a hook that modifies a color system when it is constructed.
        ///
        /// The ID pattern is like a catalog ID, except that `*` wildcards match
        /// any ID value. For example, `product([ngon(*),*])` matches any
        /// product of an `ngon` with another color system.
        ///
        /// A list of wildcard matches is passed into the callback. Each value
        /// is always passed as a string.
        ///
        /// Inside the callback, the full ID is accessible using the special
        /// variable `#id`.
        fn add_color_system_hook(
            ctx: EvalCtx,
            (id_pattern, id_pattern_span): String,
            priority: i64,
            (callback, callback_span): Arc<FnValue>,
        ) -> () {
            let tx = tx.clone();
            cat.add_hook::<ColorSystem>(Arc::new(CatalogHook {
                id_pattern: id_pattern.parse().at(id_pattern_span)?,
                priority,
                callback: Box::new(move |color_system, args| {
                    let mut scope = Scope::default();
                    scope.special.id = Some(color_system.id.to_string().into());
                    let args = args
                        .iter()
                        .map(|v| ValueData::Str(v.to_string().into()).at(crate::BUILTIN_SPAN))
                        .collect();
                    let callback = Arc::clone(&callback);

                    let color_system_ref = Arc::clone(color_system);
                    let (new_default, new_schemes) =
                        tx.eval_blocking(Arc::new(scope), move |ctx| {
                            // IIFE to mimic try_block
                            (|| {
                                let ret = callback.call(callback_span, ctx, args, Map::new())?;
                                let map_span = ret.span;
                                let mut map = ret.unwrap_or_clone_arc::<Map>()?;
                                let new_default: Option<String> =
                                    pop_map_key::<Option<String>>(&mut map, map_span, "default")?;
                                let new_schemes: Vec<(String, PerColor<PaletteColor>)> =
                                    pop_map_key::<Option<Arc<Map>>>(&mut map, map_span, "schemes")?
                                        .unwrap_or_default()
                                        .iter()
                                        .map(|(k, v)| {
                                            Ok((
                                                k.to_string(),
                                                color_scheme_from_map(
                                                    &color_system_ref.names,
                                                    v.as_ref()?,
                                                )?,
                                            ))
                                        })
                                        .try_collect()?;
                                expect_end_of_map(map, map_span)?;
                                drop(color_system_ref);
                                Ok((new_default, new_schemes))
                            })()
                            .map_err(|e| {
                                ctx.runtime.report_diagnostic(e);
                                eyre!("error executing color system hook")
                            })
                        })?;

                    let color_system_mut = Arc::get_mut(color_system)
                        .ok_or_eyre("cannot run color system hook on redirected color system")?;

                    color_system_mut.schemes.extend(new_schemes);

                    if let Some(new_default) = new_default {
                        if !color_system_mut.schemes.contains_key(&new_default) {
                            bail!("color system has no scheme {new_default:?}");
                        }
                        color_system_mut.default_scheme = new_default;
                    }

                    Ok(())
                }),
            }))
            .at(ctx.caller_span)?;
        }
    ])?;

    Ok(())
}

// TODO: doesn't really need runtime. just needs to be able to report warnings
fn color_system_from_kwargs(
    build_ctx: BuildCtx,
    caller_span: Span,
    runtime: &mut Runtime,
    kwargs: Map,
) -> Result<Arc<ColorSystem>> {
    let id = build_ctx.id();
    unpack_kwargs!(
        kwargs,
        name: String = {
            runtime.warn_at(
                caller_span,
                eco_format!("missing `name` for color system `{id}`"),
            );
            id.to_string()
        },
        colors: Value,
        schemes: Arc<Map> = Arc::new(Map::new()),
        default_scheme: Option<String>,
    );

    let mut schemes: Vec<(String, &Map)> = schemes
        .iter()
        .map(|(k, v)| Ok((k.to_string(), v.as_ref::<Map>()?)))
        .try_collect()?;

    // Build & validate names.
    let names_list = if colors.is_null() {
        // Infer color names from schemes
        schemes
            .iter()
            .flat_map(|(_, m)| m.keys().map(|s| s.as_str().into()))
            .collect::<IndexSet<Str>>()
            .into_iter()
            .collect()
    } else if colors.is::<List>() {
        colors.to::<Vec<Str>>()?
    } else if colors.is::<Map>() {
        if !schemes.is_empty() {
            return Err("`colors` cannot be a map when `schemes` is also supplied".at(caller_span));
        }
        let m = colors.as_ref()?;
        schemes.push((hyperpuzzle_core::DEFAULT_COLOR_SCHEME_NAME.to_string(), m));
        m.keys().map(|s| s.as_str().into()).collect()
    } else {
        return Err(
            colors.type_error(Type::Null | Type::List(Some(Box::new(Type::Str))) | Type::Map)
        );
    };
    let names = Names::new_simple(PerColor::from(names_list)).at(caller_span)?;

    let default_scheme = default_scheme
        .or_else(|| schemes.first().map(|(k, _)| k.to_string()))
        .unwrap_or_else(|| hyperpuzzle_core::DEFAULT_COLOR_SCHEME_NAME.to_string());

    // Build color schemes.
    let mut ret_schemes = IndexMap::new();
    for (scheme_name, m) in schemes {
        ret_schemes.insert(scheme_name.to_string(), color_scheme_from_map(&names, m)?);
    }

    if ret_schemes.is_empty() {
        ret_schemes.insert(
            default_scheme.to_string(),
            names.list().map_ref(|_, _| PaletteColor::Unknown),
        );
    }

    if !ret_schemes.contains_key(&default_scheme) {
        runtime.warn_at(
            caller_span,
            format!("default color scheme {default_scheme:?} does not exist"),
        );
    }

    Ok(Arc::new(ColorSystem {
        id: build_ctx.id().clone(),
        name,
        components: ComponentList::new(),
        names,
        schemes: ret_schemes,
        default_scheme,
        orbits: vec![],
    }))
}

fn color_scheme_from_map(names: &Names<Color>, m: &Map) -> Result<PerColor<PaletteColor>> {
    let mut scheme_values = names.list().map_ref(|_, _| PaletteColor::Unknown);
    for (k, v) in m {
        let color = names
            .lookup(k)
            .ok_or_else(|| format!("no color named {k:?}").at(v.span))?;
        let palette_color = PaletteColor::from_str(v.as_ref()?).at(v.span)?;
        scheme_values[color] = palette_color;
    }
    Ok(scheme_values)
}
