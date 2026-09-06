use std::sync::Arc;

use eyre::{OptionExt, eyre};
use hypergroup::GenSeq;
use hypermath::collections::RangeMap;
use hyperpuzzle_core::{CatalogId, Puzzle, PuzzleListEntry, TagSet, TagValue};
use hyperpuzzle_impl_nd_euclid::hps::HpsSymmetry;
use hyperpuzzlescript::builtins::catalog::tags::tags_from_map;
use hyperpuzzlescript::engine::HpsEngineError;
use hyperpuzzlescript::util::{ListOrVal, pop_map_key_in_special_var};
use hyperpuzzlescript::{
    BUILTIN_SPAN, ErrorExt, EvalCtx, FnValue, HpsEngine, Map, Result, Scope, Spanned, SpecialVar,
    Value, ValueData, pop_kwarg, unpack_kwargs,
};
use hypuz_notation::Layer;
use itertools::Itertools;
use parking_lot::Mutex;

use crate::builder::*;
use crate::{CutDistances, NamedPointOrbitSpec, PerAxisOrbit};

pub struct SymmetricPuzzleEngine;

impl HpsEngine for SymmetricPuzzleEngine {
    fn add_catalog_entries(
        &self,
        catalog: &hyperpuzzle_core::prelude::CatalogBuilder,
        eval_tx: &hyperpuzzlescript::EvalRequestTx,
        ctx: &mut EvalCtx<'_>,
        mut hps_gen: hyperpuzzlescript::engine::HpsGenerator,
    ) -> Result<(), HpsEngineError> {
        let caller_span = ctx.caller_span;

        let id = &hps_gen.id;
        if hps_gen.names.is_empty() {
            ctx.warn_at(
                caller_span,
                format!("missing `name` for puzzle generator `{id}`"),
            );
            hps_gen.names.push(id.to_string());
        }
        let name = hps_gen.names[0].clone();
        let aliases = hps_gen.names[1..].to_vec();

        let is_generator = hps_gen.gen_fn.is_some();
        let tags = get_tags(ctx, &mut hps_gen.kwargs, is_generator)?;

        let generator_list_entry = Arc::new(PuzzleListEntry {
            id: CatalogId::new(id.clone(), vec![], None),
            version: None,
            name,
            aliases,
            tags: tags.clone(),
        });

        catalog.add::<PuzzleListEntry>(hps_gen.make_generator_with_empty(
            eval_tx,
            generator_list_entry,
            move |build_ctx, tx, mut kwargs| {
                let id = build_ctx.id().clone();
                pop_kwarg!(kwargs, name: ListOrVal<String>);
                let tags = if is_generator {
                    tx.eval_blocking(Scope::new(), move |ctx| get_tags(ctx, &mut kwargs, false))??
                } else {
                    tags.clone()
                };

                let mut aliases = name.0;
                if aliases.is_empty() {
                    build_ctx.warn_fn()(eyre!("missing `name` for puzzle `{id}`"));
                    aliases.push(id.to_string());
                }
                let name = aliases.remove(0);

                Ok(Arc::new(PuzzleListEntry {
                    id,
                    version: None,
                    name,
                    aliases,
                    tags,
                }))
            },
        ))?;

        catalog.add::<PuzzleProduct>(hps_gen.make_generator(
            eval_tx,
            move |build_ctx, tx, kwargs| {
                let id = build_ctx.id();
                let meta = build_ctx.build_blocking::<PuzzleListEntry>(id)?;

                // TODO: error message on extra param says "unused function arg" but should say
                // "unused map key"
                unpack_kwargs!(
                    kwargs,
                    name: ListOrVal<String>,
                    tags: Option<Arc<Map>>,
                    twists: Option<String>,
                    colors: Option<Spanned<String>>,
                    ndim: Option<u8>,
                    (build, build_span): Arc<FnValue>,
                );

                drop((name, tags)); // already handled by PuzzleListEntry

                let id = meta.id.clone();
                let name = meta.name.clone();

                let twists = if let Some(twists) = twists {
                    Some(build_ctx.build_str_blocking::<TwistSystemProduct>(&twists)?)
                } else {
                    None
                };

                let ndim = ndim
                    .or(twists.as_ref().map(|t| t.ndim()))
                    .ok_or_eyre("at least one of `ndim` and `twists` is required")?;

                let mut scope = Scope::default();
                scope.special.id = Some(id.to_string().into());
                scope.special.ndim = Some(ndim);
                scope.special.shape = Arc::new(Mutex::new({
                    let mut m = Map::new();
                    m.insert("points".into(), super::new_hps_list());
                    m.insert("facets".into(), super::new_hps_list());
                    ValueData::Map(Arc::new(m)).at(BUILTIN_SPAN)
                }));
                scope.special.puz = Arc::new(Mutex::new({
                    let mut m = Map::new();
                    m.insert("cuts".into(), super::new_hps_map());
                    m.insert("is_full_cut".into(), super::new_hps_map());
                    ValueData::Map(Arc::new(m)).at(BUILTIN_SPAN)
                }));
                tx.eval_blocking(Arc::new(scope), move |ctx| {
                    build.call(build_span, ctx, vec![], Map::new())?;

                    let mut shape_map = Arc::unwrap_or_clone(
                        std::mem::take(&mut *ctx.scope.special.shape.lock()).to::<Arc<Map>>()?,
                    );

                    let (sym, sym_span) = pop_map_key_in_special_var::<Spanned<HpsSymmetry>>(
                        &mut shape_map,
                        build_span,
                        SpecialVar::Shape,
                        "sym",
                    )?;
                    let generators = sym
                        .generators()
                        .map_ref(|g, m| (GenSeq::new([g]), m.clone()));

                    build_ctx.push_task("parsing named points specification");
                    let mut autonames = crate::named_point_autonames();
                    let named_point_orbits: Vec<NamedPointOrbitSpec> =
                        pop_map_key_in_special_var::<Vec<Value>>(
                            &mut shape_map,
                            build_span,
                            SpecialVar::Shape,
                            "points",
                        )?
                        .into_iter()
                        .map(|value| {
                            super::named_point_orbit_from_value(
                                ctx,
                                &generators,
                                value,
                                &mut autonames,
                            )
                        })
                        .try_collect()?;
                    build_ctx.pop_task();

                    build_ctx.push_task("parsing facets specification");
                    let facet_orbits: Vec<_> =
                        super::simple_orbit_from_value(pop_map_key_in_special_var::<Vec<Value>>(
                            &mut shape_map,
                            build_span,
                            SpecialVar::Shape,
                            "facets",
                        )?)?;
                    build_ctx.pop_task();

                    let mut puz_map = Arc::unwrap_or_clone(
                        std::mem::take(&mut *ctx.scope.special.puz.lock()).to::<Arc<Map>>()?,
                    );

                    let axis_orbit_count = match &twists {
                        Some(t) => t.axis_orbits().count(),
                        None => 0,
                    };

                    build_ctx.push_task("parsing negative layer specs");
                    let mut opt_negative_layers =
                        PerAxisOrbit::<Option<bool>>::new_with_len(axis_orbit_count);
                    let (negative_layers_spec, negative_layers_spec_span) =
                        pop_map_key_in_special_var::<Spanned<Arc<Map>>>(
                            &mut puz_map,
                            build_span,
                            SpecialVar::Puz,
                            "is_full_cut",
                        )?;
                    if let Some(twists) = &twists {
                        for (k, v) in &*negative_layers_spec {
                            let axis = twists
                                .axis_from_name(k)
                                .ok_or_else(|| format!("no axis named {k:?}"))
                                .at(v.span)?;
                            let i = twists
                                .orbit_containing_axis(axis)
                                .ok_or("axis has no orbit")
                                .at(v.span)?;
                            if opt_negative_layers[i].is_some() {
                                Err("duplicate is_full_cut for axis orbit".at(v.span))?;
                            }
                            opt_negative_layers[i] = Some(v.ref_to()?);
                        }
                    } else if !negative_layers_spec.is_empty() {
                        ctx.warn_at(
                            negative_layers_spec_span,
                            "ignoring `is_full_cut` because there are no axes",
                        );
                    }
                    let axis_orbit_is_full_cut =
                        opt_negative_layers.map(|_, opt| opt.unwrap_or(false));
                    build_ctx.pop_task();

                    build_ctx.push_task("parsing cuts specification");
                    let (cuts_spec, cuts_spec_span) =
                        pop_map_key_in_special_var::<Spanned<Arc<Map>>>(
                            &mut puz_map,
                            build_span,
                            SpecialVar::Puz,
                            "cuts",
                        )?;
                    let mut opt_cut_distances =
                        PerAxisOrbit::<Option<CutDistances>>::new_with_len(axis_orbit_count);
                    if let Some(twists) = &twists {
                        for (k, v) in &*cuts_spec {
                            let axis = twists
                                .axis_from_name(k)
                                .ok_or_else(|| format!("no axis named {k:?}"))
                                .at(v.span)?;
                            let i = twists
                                .orbit_containing_axis(axis)
                                .ok_or("axis has no orbit")
                                .at(v.span)?;
                            if opt_cut_distances[i].is_some() {
                                Err("duplicate cuts for axis orbit".at(v.span))?;
                            }
                            opt_cut_distances[i] = Some(CutDistances::new(v.ref_to()?).at(v.span)?);
                        }
                    } else if !cuts_spec.is_empty() {
                        ctx.warn_at(cuts_spec_span, "ignoring `cuts` because there are no axes");
                    }
                    let axis_orbit_cut_distances =
                        opt_cut_distances.map(|_, opt| opt.unwrap_or_default());
                    build_ctx.pop_task();

                    build_ctx.push_task("building layer maps");
                    let opt_layer_maps =
                        PerAxisOrbit::<Option<RangeMap<Option<Layer>>>>::new_with_len(
                            axis_orbit_count,
                        );
                    // TODO: parse layer specs
                    let axis_orbit_layer_maps = opt_layer_maps.try_map(|orbit_index, opt| {
                        opt.map(Ok).unwrap_or_else(|| {
                            axis_orbit_cut_distances[orbit_index]
                                .implied_layers(axis_orbit_is_full_cut[orbit_index])
                                .at(cuts_spec_span)
                        })
                    })?;
                    build_ctx.pop_task();

                    Ok(Arc::new(
                        PuzzleProduct::new_factor(
                            &build_ctx,
                            &crate::FactorPuzzleSpec {
                                id,
                                name,
                                symmetry: sym.isometry_group().at(sym_span)?,
                                coxeter_matrix: sym.as_coxeter().cloned(),
                                named_point_orbits,
                                facet_orbits,
                                colors_id: colors
                                    .map(|(s, span)| s.parse().at(span))
                                    .transpose()?,
                                twists,
                                axis_orbit_cut_distances,
                                axis_orbit_layer_maps,
                                axis_orbit_is_full_cut,
                            },
                            &mut build_ctx.warn_fn(),
                        )
                        .at(caller_span)?,
                    ))
                })?
            },
        ))?;

        catalog.add::<Puzzle>(hps_gen.make_generator(eval_tx, |build_ctx, _tx, _kwargs| {
            Ok(crate::build_product_puzzle_impl(build_ctx)?)
        }))?;

        catalog.add_generator_to_puzzle_list(id);

        Ok(())
    }
}

fn get_tags(
    ctx: &mut EvalCtx<'_>,
    kwargs: &mut Map,
    is_generator: bool,
) -> Result<TagSet, HpsEngineError> {
    let mut tags = match kwargs.get("tags") {
        Some(v) if !v.is_null() => tags_from_map(ctx, Arc::clone(v.as_ref()?)),
        _ => TagSet::new(),
    };
    if !is_generator {
        kwargs.swap_remove("tags");
    }

    // IIFE to mimic try_block
    (|| {
        if is_generator {
            tags.insert_named("generator", true.into())?;
        }
        tags.insert_named("solid", true.into())?;
        tags.insert_named("doctrinaire", true.into())?;
        tags.insert_named("pseudodoctrinaire", true.into())?;
        if let Some(v) = kwargs.get("ndim")
            && let Ok(ndim) = v.ref_to::<i64>()
        {
            tags.insert_named("ndim", TagValue::Int(ndim))?;
        }
        if let Some(v) = kwargs.get("twists")
            && let Ok(twists) = v.as_ref::<str>()
        {
            tags.insert_named("twists", TagValue::Str(twists.to_owned()))?;
        }
        if let Some(v) = kwargs.get("colors")
            && let Ok(colors) = v.as_ref::<str>()
        {
            tags.insert_named("colors", TagValue::Str(colors.to_owned()))?;
        }
        eyre::Ok(())
    })()?;

    Ok(tags)
}
