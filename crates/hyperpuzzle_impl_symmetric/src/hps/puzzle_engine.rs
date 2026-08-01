use std::sync::Arc;

use eyre::eyre;
use hypergroup::GenSeq;
use hypermath::Float;
use hyperpuzzle_core::{CatalogId, ColorSystem, Puzzle, PuzzleListEntry, TagSet};
use hyperpuzzle_impl_nd_euclid::hps::HpsSymmetry;
use hyperpuzzlescript::{
    BUILTIN_SPAN, ErrorExt, EvalCtx, FnValue, HpsEngine, Map, Result, Scope, Spanned, SpecialVar,
    Value, ValueData, engine::HpsEngineError, pop_kwarg, unpack_kwargs,
    util::pop_map_key_in_special_var,
};
use itertools::Itertools;
use parking_lot::Mutex;

use crate::{CutDistances, builder::*, spec::FacetOrbitSpec};

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
        pop_kwarg!(hps_gen.kwargs, name: String = {
            ctx.warn_at(
                caller_span,
                format!("missing `name` for puzzle generator `{id}`"),
            );
            id.to_string()
        });

        let generator_list_entry = Arc::new(PuzzleListEntry {
            id: CatalogId::new(id.clone(), vec![], None),
            version: None,
            name,
            aliases: vec![], // TODO
            tags: TagSet::todo(),
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
                Ok(Arc::new(PuzzleListEntry {
                    id,
                    version: None,
                    name,
                    aliases: vec![],
                    tags: TagSet::todo(),
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
                    twists: String,
                    colors: String,
                    ndim: u8,
                    (build, build_span): Arc<FnValue>,
                );

                let id = meta.id.clone();
                let name = meta.name.clone();

                let colors = Arc::new(ColorSystemDisjointUnion::from_color_system(
                    build_ctx.build_str_blocking::<ColorSystem>(&colors)?,
                ));
                let twists = build_ctx.build_str_blocking::<TwistSystemProduct>(&twists)?;

                let mut scope = Scope::default();
                scope.special.id = Some(id.to_string().into());
                scope.special.ndim = Some(ndim);
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

                    let facet_orbits: Vec<FacetOrbitSpec> =
                        pop_map_key_in_special_var::<Vec<Value>>(
                            &mut shape_map,
                            build_span,
                            SpecialVar::Shape,
                            "facets",
                        )?
                        .into_iter()
                        .map(|value| super::named_orbit_from_value(ctx, &generators, value))
                        .map_ok(|named_facet_poles| FacetOrbitSpec { named_facet_poles })
                        .try_collect()?;

                    let mut puz_map = Arc::unwrap_or_clone(
                        std::mem::take(&mut *ctx.scope.special.puz.lock()).to::<Arc<Map>>()?,
                    );

                    let mut axis_orbit_cut_distances = vec![None; twists.axis_orbits.len()];
                    for (k, v) in &*pop_map_key_in_special_var::<Arc<Map>>(
                        &mut puz_map,
                        build_span,
                        SpecialVar::Puz,
                        "layers",
                    )? {
                        let k = k.to_string();
                        let Some(i) = twists.axis_orbits.iter().position(|o| o.names.contains(&k))
                        else {
                            ctx.warn_at(v.span, format!("no axis named {k:?}"));
                            continue;
                        };
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
                            colors,
                            twists,
                            &axis_orbit_cut_distances,
                            &mut build_ctx.warn_fn(),
                        )
                        .at(caller_span)?,
                    ))
                })
            },
        ))?;

        catalog.add::<Puzzle>(hps_gen.make_generator(eval_tx, |build_ctx, tx, kwargs| {
            Ok(build_ctx
                .build_blocking::<PuzzleProduct>(build_ctx.id())?
                .build(&build_ctx, &mut build_ctx.warn_fn())?)
        }))?;

        Ok(())
    }
}
