use std::str::FromStr;
use std::sync::Arc;

use ecow::eco_format;
use hyperpuzzle_core::{
    BuildCtx, CatalogBuilder, ColorSystem, ComponentList, NameSpecBiMapBuilder, PaletteColor,
    PerColor,
};
use indexmap::IndexMap;

use crate::util::pop_map_key;
use crate::{Builtins, ErrorExt, EvalRequestTx, List, Map, Result, Runtime, Span, Spanned, Str};

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
            let hps_gen = super::generators::hps_generator_from_kwargs(kwargs)?;
            let caller_span = ctx.caller_span;
            cat.add(hps_gen.make_generator(&tx, move |build_ctx, tx, kwargs| {
                Ok(tx.eval_blocking_raw(move |runtime| {
                    color_system_from_kwargs(build_ctx, caller_span, runtime, kwargs)
                })?)
            }))
            .at(caller_span)?;
        }
    ])
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

        let id = display_names.next_idx().at(caller_span)?;

        let (name_spec, name_span): Spanned<String> = pop_map_key(&mut map, map_span, "name")?;
        names.set(id, Some(name_spec.clone())).at(name_span)?;

        let display = pop_map_key::<Option<_>>(&mut map, map_span, "display")?
            .unwrap_or_else(|| hyperpuzzle_core::preferred_name_from_name_spec(&name_spec));
        display_names.push(display).at(map_span)?;

        let default_color =
            match pop_map_key::<Option<Spanned<Str>>>(&mut map, map_span, "default")? {
                None => PaletteColor::Unknown,
                Some((s, span)) => {
                    any_color_has_default = true;
                    PaletteColor::from_str(&s).at(span)?
                }
            };
        default_scheme.push(default_color).at(caller_span)?;
    }

    let names = names
        .build(display_names.len())
        .ok_or_else(|| "missing color name".at(caller_span))?;

    // Add color schemes.
    let mut ret_schemes = IndexMap::new();
    if let Some(color_schemes_list) = schemes {
        if any_color_has_default {
            runtime.warn_at(
                caller_span,
                "per-color `default` is ignored when used with `schemes`",
            );
        }

        for (mut map, map_span) in color_schemes_list {
            if map.len() != 2 {
                return Err("expected list with 2 elements".at(map_span));
            }
            let scheme_name = std::mem::take(&mut map[0]).to::<String>()?;
            let mut scheme_values = PerColor::<PaletteColor>::new_with_len(display_names.len());
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
        display_names,
        schemes: ret_schemes,
        default_scheme,
        orbits: vec![],
    }))
}
