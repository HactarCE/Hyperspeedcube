use std::sync::Arc;

use eyre::eyre;
use hypergroup::GenSeq;
use hypermath::Float;
use hyperpuzzle_core::{CatalogId, Puzzle, PuzzleListEntry};
use hyperpuzzle_impl_nd_euclid::hps::HpsSymmetry;
use hyperpuzzlescript::{
    BUILTIN_SPAN, ErrorExt, EvalCtx, FnValue, HpsEngine, Map, Result, Scope, Spanned, SpecialVar,
    Value, ValueData, builtins::catalog::tags::tags_from_map, engine::HpsEngineError, pop_kwarg,
    unpack_kwargs, util::pop_map_key_in_special_var,
};
use itertools::Itertools;
use parking_lot::Mutex;

use crate::{CutDistances, builder::*};

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
        let name = hps_gen.name.clone().unwrap_or_else(|| {
            ctx.warn_at(
                caller_span,
                format!("missing `name` for puzzle generator `{id}`"),
            );
            id.to_string()
        });

        pop_kwarg!(hps_gen.kwargs, aliases: Vec<String> = vec![]);
        pop_kwarg!(hps_gen.kwargs, tags: Option<Arc<Map>>);

        let mut tags = tags.map(|m| tags_from_map(ctx, m)).unwrap_or_default();
        // IIFE to mimic try_block
        (|| {
            if hps_gen.gen_fn.is_some() {
                tags.insert_named("generator", true.into())?;
            }
            tags.insert_named("solid", true.into())?;
            tags.insert_named("doctrinaire", true.into())?;
            tags.insert_named("pseudodoctrinaire", true.into())?;
            if let Some(v) = hps_gen.kwargs.get("ndim")
                && let Ok(ndim) = v.ref_to::<i64>()
            {
                tags.insert_named("ndim", ndim.into())?;
            }
            eyre::Ok(())
        })()?;

        let generator_list_entry = Arc::new(PuzzleListEntry {
            id: CatalogId::new(id.clone(), vec![], None),
            version: None,
            name,
            aliases,
            tags,
        });

        catalog.add::<PuzzleListEntry>(hps_gen.make_generator_with_empty(
            eval_tx,
            generator_list_entry,
            move |build_ctx, tx, mut kwargs| {
                let id = build_ctx.id().clone();
                pop_kwarg!(kwargs, name: String = {
                    build_ctx.warn_fn()(eyre!("missing `name` for puzzle `{id}`"));
                    id.to_string()
                });
                pop_kwarg!(kwargs, aliases: Vec<String> = vec![]);
                pop_kwarg!(kwargs, tags: Option<Arc<Map>>);

                let mut tags = tags
                    .map(|m| tx.eval_blocking(Scope::new(), |ctx| tags_from_map(ctx, m)))
                    .unwrap_or_default();
                tags.insert_named("solid", true.into())
                    .map_err(|e| eyre!(e))?;
                tags.insert_named("doctrinaire", true.into())
                    .map_err(|e| eyre!(e))?;
                tags.insert_named("pseudodoctrinaire", true.into())
                    .map_err(|e| eyre!(e))?;

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

                // TODO: error message on extra param says "unused function arg" but should say "unused map key"
                unpack_kwargs!(
                    kwargs,
                    name: String = {
                        build_ctx.warn_fn()(eyre!("missing `name` for puzzle `{id}`"));
                        id.to_string()
                    },
                    twists: Option<String>,
                    (colors, colors_span): String,
                    ndim: Option<u8>,
                    tags: Option<Arc<Map>>,
                    (build, build_span): Arc<FnValue>,
                );

                let id = meta.id.clone();
                let name = meta.name.clone();

                let twists = if let Some(twists) = twists {
                    build_ctx.build_str_blocking::<TwistSystemProduct>(&twists)?
                } else if let Some(ndim) = ndim {
                    build_ctx.build_str_blocking::<TwistSystemProduct>(&format!("empty({ndim})"))?
                } else {
                    Err(eyre!("at least one of `ndim` and `twists` is required"))?
                };
                let twists_ndim = twists.ndim();
                if let Some(expected_ndim) = ndim
                    && twists_ndim != expected_ndim
                {
                    Err(eyre!(
                        "twist system has ndim={twists_ndim:?} \
                         but expected ndim={expected_ndim:?}"
                    ))?;
                }

                let mut scope = Scope::default();
                scope.special.id = Some(id.to_string().into());
                scope.special.ndim = Some(twists.ndim());
                scope.special.shape = Arc::new(Mutex::new({
                    let mut m = Map::new();
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

                    let facet_orbits: Vec<_> = pop_map_key_in_special_var::<Vec<Value>>(
                        &mut shape_map,
                        build_span,
                        SpecialVar::Shape,
                        "facets",
                    )?
                    .into_iter()
                    .map(|value| super::named_orbit_from_value(ctx, &generators, value))
                    .try_collect()?;

                    let mut puz_map = Arc::unwrap_or_clone(
                        std::mem::take(&mut *ctx.scope.special.puz.lock()).to::<Arc<Map>>()?,
                    );

                    let mut axis_orbit_cut_distances = vec![None; twists.axis_orbits().count()];
                    for (k, v) in &*pop_map_key_in_special_var::<Arc<Map>>(
                        &mut puz_map,
                        build_span,
                        SpecialVar::Puz,
                        "layers",
                    )? {
                        let axis = twists
                            .axis_from_name(k)
                            .ok_or_else(|| format!("no axis named {k:?}"))
                            .at(v.span)?;
                        let i = twists
                            .orbit_containing_axis(axis)
                            .ok_or("axis has no orbit")
                            .at(v.span)?;
                        if axis_orbit_cut_distances[i].is_some() {
                            ctx.warn_at(
                                v.span,
                                format!("duplicate layers for orbit of axis {k:?}"),
                            );
                        }
                        axis_orbit_cut_distances[i] = Some(v.ref_to::<Vec<Float>>()?);
                    }
                    let axis_orbit_cut_distances = axis_orbit_cut_distances
                        .into_iter()
                        .map(|cut_distances| CutDistances(cut_distances.unwrap_or_default()))
                        .collect_vec();

                    Ok(Arc::new(
                        PuzzleProduct::new_factor(
                            id,
                            name,
                            coxeter_matrix,
                            &facet_orbits,
                            colors.parse().at(colors_span)?,
                            &twists,
                            &axis_orbit_cut_distances,
                            &mut build_ctx.warn_fn(),
                        )
                        .at(caller_span)?,
                    ))
                })
            },
        ))?;

        catalog.add::<Puzzle>(hps_gen.make_generator(eval_tx, |build_ctx, tx, kwargs| {
            Ok(crate::build_product_puzzle_impl(build_ctx)?)
        }))?;

        Ok(())
    }
}
