use std::sync::Arc;

use eyre::{Context, bail};
use hypergroup::{AbbrGenSeq, CoxeterMatrix, GenSeq, IsometryGroup};
use hypermath::{Float, Vector, pga::Motor};
use hyperpuzzle_core::{
    CatalogMetadata, Component, ComponentList, Puzzle, Redirectable, TwistSystem,
    catalog::{BuildCtx, BuildFn, Generator, GeneratorOutput},
    util::MaybeAdHoc,
};
use hyperpuzzle_impl_nd_euclid::hps::{HpsOrbitNames, HpsSymmetry};
use hyperpuzzlescript::{
    BUILTIN_SPAN, ErrorExt, EvalCtx, FnValue, List, Map, NonEmptyList, NonEmptyVec, Result, Scope,
    Spanned, Value, ValueData, unpack_kwargs, util::pop_map_key,
};
use hypuz_notation::Str;
use itertools::Itertools;
use parking_lot::Mutex;

use super::HpsSymmetric;
use crate::{
    AxisOrbitSpec, CutDistances, FactorPuzzleSpec, NamedPointOrbitSpec, ProductPuzzleSpec,
    builder::*, spec::FacetOrbitSpec,
};

impl hyperpuzzlescript::EngineCallback<Puzzle> for HpsSymmetric {
    fn new(
        &self,
        ctx: &mut hyperpuzzlescript::EvalCtx<'_>,
        mut meta: hyperpuzzle_core::CatalogMetadata,
        kwargs: hyperpuzzlescript::Map,
        eval_tx: hyperpuzzlescript::EvalRequestTx,
    ) -> Result<GeneratorOutput<Puzzle>> {
        let caller_span = ctx.caller_span;

        unpack_kwargs!(
            kwargs,
            (twists, twists_span): Option<String>,
            (colors, colors_span): Option<String>,
            ndim: u8,
            (build, build_span): Arc<FnValue>,
        );

        meta.tags.set_opt_color_system(colors.as_deref());
        meta.tags.set_opt_twist_system(twists.as_deref());
        meta.tags.set_ndim(ndim as i64);

        let meta = Arc::new(meta);
        let m = Arc::clone(&meta);

        let factor_puzzle_build_fn = move |build_ctx: &BuildCtx| {
            let logger = &build_ctx.catalog.logger;
            let id = meta.id.clone();
            let name = meta.name.clone();

            if colors.is_none() {
                logger.warn(format!("using ad-hoc color system for puzzle {id:?}"));
            }
            if twists.is_none() {
                logger.warn(format!("using ad-hoc twist system for puzzle {id:?}"));
            }

            let mut scope = Scope::default();
            scope.special.id = Some(id.to_string().into());
            scope.special.ndim = Some(ndim);
            scope.special.shape = Arc::new(Mutex::new({
                let mut m = Map::new();
                m.insert("facets".into(), super::new_hps_list());
                ValueData::Map(Arc::new(m)).at(BUILTIN_SPAN)
            }));
            if twists.is_none() {
                super::twist_system_engine::init_twists_in_hps_scope(&mut scope);
            }
            scope.special.puz = Arc::new(Mutex::new({
                let mut m = Map::new();
                m.insert("layers".into(), super::new_hps_map());
                ValueData::Map(Arc::new(m)).at(BUILTIN_SPAN)
            }));
            let scope = Arc::new(scope);

            build_ctx.set_building::<Puzzle>();

            let build_fn = Arc::clone(&build);
            let colors = colors
                .as_ref()
                .map(|s| build_ctx.build_str_blocking(s))
                .transpose()?;
            let twists = twists
                .as_ref()
                .map(|s| build_ctx.build_str_blocking(s))
                .transpose()?;
            let build_ctx = build_ctx.clone();
            eval_tx.eval_blocking(move |runtime| {
                // IIFE to mimic try_block
                (|| {
                    let mut _exports = None;
                    let mut ctx = EvalCtx::new(&scope, runtime, caller_span, &mut _exports);

                    build_fn.call(build_span, &mut ctx, vec![], Map::new())?;

                    let mut shape_map = Arc::unwrap_or_clone(
                        std::mem::take(&mut *scope.special.shape.lock()).to::<Arc<Map>>()?,
                    );

                    // TODO: improve error message when missing one of these
                    let (sym, sym_span) =
                        pop_map_key::<Spanned<HpsSymmetry>>(&mut shape_map, BUILTIN_SPAN, "sym")?;
                    let coxeter_matrix = sym.as_coxeter(sym_span)?.clone();
                    let generators = coxeter_matrix
                        .generator_motors()
                        .at(sym_span)?
                        .map(|g, m| (GenSeq::new([g]), m));

                    let facet_orbits: Vec<FacetOrbitSpec> =
                        pop_map_key::<Vec<Value>>(&mut shape_map, BUILTIN_SPAN, "facets")?
                            .into_iter()
                            .map(|value| {
                                super::named_orbit_from_value(&mut ctx, &generators, value)
                            })
                            .map_ok(|named_facet_poles| FacetOrbitSpec { named_facet_poles })
                            .try_collect()?;

                    let colors = match colors {
                        Some(colors) => colors,
                        None => ColorSystemDisjointUnion {
                            summand_ids: vec![hyperpuzzle_core::ad_hoc_id(id.clone())],
                            summand_names: vec![format!("{name} (ad-hoc)")],
                            orbits: facet_orbits
                                .iter()
                                .map(|orbit| {
                                    orbit
                                        .named_facet_poles
                                        .iter()
                                        .map(|(_, color_name, _)| Some(color_name.clone()))
                                        .collect()
                                })
                                .collect(),
                        }
                        .build(&build_ctx, &mut ctx.warnf())
                        .context("building ad-hoc color system")
                        .at(caller_span)?,
                    };

                    let twists = match twists {
                        Some(twists) => twists,
                        None => super::twist_system_engine::twists_builder_from_hps(
                            &mut ctx,
                            hyperpuzzle_core::ad_hoc_id(id.clone()),
                            format!("{id} (ad-hoc)"),
                        )?
                        .build(&build_ctx, &mut ctx.warnf())
                        .context("building ad-hoc twist system")
                        .at(caller_span)?,
                    };

                    let mut puz_map = Arc::unwrap_or_clone(
                        std::mem::take(&mut *ctx.scope.special.puz.lock()).to::<Arc<Map>>()?,
                    );

                    let axis_orbits = &twists
                        .components
                        .get::<TwistSystemProduct>()
                        .at(twists_span)?
                        .axis_orbits;

                    let mut axis_orbit_cut_distances = vec![None; axis_orbits.len()];
                    for (k, v) in &*pop_map_key::<Arc<Map>>(&mut puz_map, BUILTIN_SPAN, "layers")? {
                        let k = k.to_string();
                        let Some(i) = axis_orbits.iter().position(|o| o.names.contains(&k)) else {
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

                    PuzzleProduct::new_factor(
                        id,
                        name,
                        coxeter_matrix,
                        &facet_orbits,
                        MaybeAdHoc::Fixed(colors),
                        MaybeAdHoc::Fixed(twists),
                        &axis_orbit_cut_distances,
                        &mut ctx.warnf(),
                    )
                    .at(caller_span)
                })()
                .map_err(|e| runtime.report_and_convert_to_eyre(e))
                .wrap_err("error building puzzle")
            })
        };

        Ok(
            Arc::new(PuzzleProductBuildFn(Box::new(factor_puzzle_build_fn)))
                .into_generator_output(m),
        )
    }
}

pub struct PuzzleProductBuildFn(
    pub Box<dyn Send + Sync + for<'a> Fn(&'a BuildCtx) -> eyre::Result<PuzzleProduct>>,
);

impl Component<GeneratorOutput<Puzzle>> for PuzzleProductBuildFn {}

impl PuzzleProductBuildFn {
    pub fn into_generator_output(
        self: Arc<Self>,
        meta: Arc<CatalogMetadata>,
    ) -> GeneratorOutput<Puzzle> {
        let build = self.build_fn();
        let mut components = ComponentList::new();
        components.insert(self);
        GeneratorOutput {
            meta,
            components,
            build,
        }
    }

    fn build_fn(self: &Arc<Self>) -> BuildFn<Puzzle> {
        let this = Arc::clone(self);
        Arc::new(move |build_ctx| {
            let mut warn_fn = |e| build_ctx.catalog.logger.warn(format!("{e:?}"));
            this.0(&build_ctx)?.build(&build_ctx, &mut warn_fn)
        })
    }
}
