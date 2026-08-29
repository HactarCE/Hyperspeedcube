use std::sync::Arc;

use eyre::{OptionExt, eyre};
use hypergroup::GenSeq;
use hypermath::Float;
use hyperpuzzle_core::{CatalogId, Puzzle, PuzzleListEntry, TagSet, TagValue};
use hyperpuzzle_impl_nd_euclid::hps::HpsSymmetry;
use hyperpuzzlescript::builtins::catalog::tags::tags_from_map;
use hyperpuzzlescript::engine::HpsEngineError;
use hyperpuzzlescript::util::{ListOrVal, pop_map_key_in_special_var};
use hyperpuzzlescript::{
    BUILTIN_SPAN, ErrorExt, EvalCtx, FnValue, HpsEngine, Map, Result, Scope, Spanned, SpecialVar,
    Value, ValueData, pop_kwarg, unpack_kwargs,
};
use itertools::Itertools;
use parking_lot::Mutex;

use crate::builder::*;
use crate::{CutDistances, NamedPointOrbitSpec};

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
                    m.insert("layers".into(), super::new_hps_map());
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
                    let coxeter_matrix = sym.as_coxeter(sym_span)?.clone();
                    let generators = coxeter_matrix
                        .generator_motors()
                        .at(sym_span)?
                        .map(|g, m| (GenSeq::new([g]), m));

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

                    build_ctx.push_task("parsing cut distances specification");
                    let (layers_spec, layers_spec_span) =
                        pop_map_key_in_special_var::<Spanned<Arc<Map>>>(
                            &mut puz_map,
                            build_span,
                            SpecialVar::Puz,
                            "layers",
                        )?;
                    let axis_orbit_cut_distances;
                    if let Some(twists) = &twists {
                        let mut layer_floats = vec![None; twists.axis_orbits().count()];
                        for (k, v) in &*layers_spec {
                            let axis = twists
                                .axis_from_name(k)
                                .ok_or_else(|| format!("no axis named {k:?}"))
                                .at(v.span)?;
                            let i = twists
                                .orbit_containing_axis(axis)
                                .ok_or("axis has no orbit")
                                .at(v.span)?;
                            if layer_floats[i].is_some() {
                                ctx.warn_at(
                                    v.span,
                                    format!("duplicate layers for orbit of axis {k:?}"),
                                );
                            }
                            layer_floats[i] = Some(v.ref_to::<Vec<Float>>()?);
                        }
                        axis_orbit_cut_distances = layer_floats
                            .into_iter()
                            .map(Option::unwrap_or_default)
                            .map(CutDistances::new)
                            .try_collect()?;
                    } else {
                        if !layers_spec.is_empty() {
                            ctx.warn_at(
                                layers_spec_span,
                                "ignoring `layers` because there are no axes",
                            );
                        }
                        axis_orbit_cut_distances = vec![];
                    }
                    build_ctx.pop_task();

                    Ok(Arc::new(
                        PuzzleProduct::new_factor(
                            &build_ctx,
                            &crate::FactorPuzzleSpec {
                                id,
                                name,
                                coxeter_matrix,
                                named_point_orbits,
                                facet_orbits,
                                colors_id: colors
                                    .map(|(s, span)| s.parse().at(span))
                                    .transpose()?,
                                twists,
                                axis_orbit_cut_distances,
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
