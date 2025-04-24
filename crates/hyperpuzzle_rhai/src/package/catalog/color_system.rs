//! Rhai `add_color_system()` function.

use std::sync::Arc;

use hyperpuzzle_impl_nd_euclid::builder::*;

use super::*;
use crate::util::warnf;

pub fn register(module: &mut Module, catalog: &Catalog, eval_tx: &RhaiEvalRequestTx) {
    let cat = catalog.clone();
    new_fn("add_color_system").set_into_module(
        module,
        move |ctx: Ctx<'_>, map: Map| -> Result<()> {
            let builder = color_system_from_rhai_map(&ctx, map)?;
            let color_system = builder.build(None, None, void_warn(&ctx)).eyrefmt()?;
            cat.add_color_system(Arc::new(color_system)).eyrefmt()
        },
    );

    let cat = catalog.clone();
    let tx = eval_tx.clone();
    new_fn("add_color_system_generator").set_into_module(
        module,
        move |ctx: Ctx<'_>, gen_map: Map| -> Result<()> {
            let tx = tx.clone();
            let generator = super::generator::generator_from_rhai_map(
                &ctx,
                tx,
                gen_map,
                |ctx, build_ctx, map| {
                    let builder = color_system_from_rhai_map(ctx, map)?;
                    builder
                        .build(Some(&build_ctx), None, void_warn(ctx))
                        .eyrefmt()
                },
            )?;
            cat.add_color_system_generator(Arc::new(generator))
                .eyrefmt()
        },
    );
}

/// Constructs a color system from a Rhai specification.
pub fn color_system_from_rhai_map(ctx: &Ctx<'_>, data: Map) -> Result<ColorSystemBuilder> {
    let_from_map!(ctx, data, {
        let id: String;
        let name: Option<String>;
        let colors: Vec<Map>;
        let schemes: Option<Vec<Map>>;
        let default_scheme: Option<String>;
    });
    let colors_array = colors;

    let mut colors = ColorSystemBuilder::new_shared(id);
    colors.name = name;
    colors.default_scheme = default_scheme;

    // Add colors.
    add_colors_from_array(ctx, &mut colors, colors_array, schemes.is_some())?;

    // Add color schemes.
    for scheme in schemes.unwrap_or_default() {
        let_from_map!(ctx, scheme, {
            let name: String;
            let map: Map;
        });
        add_color_scheme_from_map(ctx, &mut colors, name, map)?;
    }

    // Reset the "is modified" flag.
    colors.is_modified = false;

    Ok(colors)
}

fn add_colors_from_array(
    ctx: &Ctx<'_>,
    colors: &mut ColorSystemBuilder,
    colors_array: Vec<Map>,
    has_schemes: bool,
) -> Result {
    let mut warn_init = false;

    for color_data in colors_array {
        let_from_map!(ctx, color_data, {
            let name: Option<String>;
            let display: Option<String>;
            let init: Option<String>;
        });

        warn_init |= init.is_some() && has_schemes;

        let id = colors.add().eyrefmt()?;
        colors.names.set(id, name).or_else(warnf(ctx))?;
        colors.display_names.extend_to_contain(id);
        colors.display_names[id] = display;
        if let Some(s) = init {
            colors.set_default_color(id, Some(s.parse().eyrefmt()?));
        }
    }

    if warn_init {
        warn(ctx, "per-color 'init' cannot be used with color schemes")?;
    }

    Ok(())
}

fn add_color_scheme_from_map(
    ctx: &Ctx<'_>,
    colors: &mut ColorSystemBuilder,
    name: String,
    map: Map,
) -> Result {
    let mut new_scheme = PerColor::new();
    new_scheme.resize(colors.len()).map_err(|e| e.to_string())?;
    for (k, v) in map {
        let id = colors
            .names
            .id_from_string(&k)
            .ok_or_else(|| format!("no color with name {k:?}"))?;
        new_scheme[id] = Some(from_rhai::<String>(ctx, v)?.parse().eyrefmt()?);
    }
    colors.schemes.insert(name, new_scheme);

    Ok(())
}
