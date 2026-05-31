use std::sync::Arc;

use eyre::{Context, bail};
use hypergroup::{AbbrGenSeq, CoxeterMatrix, GenSeq, IsometryGroup};
use hypermath::{Float, Vector, pga::Motor};
use hyperpuzzle_core::{
    CatalogId, CatalogMetadata, Component, ComponentList, Puzzle, Redirectable, TwistSystem,
    catalog::{BuildCtx, BuildFn, Generator, GeneratorOutput},
};
use hyperpuzzle_impl_nd_euclid::hps::{HpsOrbitNames, HpsSymmetry};
use hyperpuzzlescript::{
    BUILTIN_SPAN, ErrorExt, EvalCtx, FnValue, List, Map, NonEmptyList, NonEmptyVec, Result, Scope,
    Span, Spanned, Value, ValueData, unpack_kwargs, util::pop_map_key,
};
use hypuz_notation::Str;
use itertools::Itertools;
use parking_lot::Mutex;

use super::HpsSymmetric;
use crate::builder::*;
use crate::{
    AxisOrbitSpec, FactorPuzzleSpec, NamedPointOrbitSpec, ProductPuzzleSpec, builder::*,
    spec::FacetOrbitSpec,
};

impl hyperpuzzlescript::EngineCallback<TwistSystem> for HpsSymmetric {
    fn new(
        &self,
        ctx: &mut hyperpuzzlescript::EvalCtx<'_>,
        meta: hyperpuzzle_core::CatalogMetadata,
        kwargs: hyperpuzzlescript::Map,
        eval_tx: hyperpuzzlescript::EvalRequestTx,
    ) -> Result<GeneratorOutput<TwistSystem>> {
        let caller_span = ctx.caller_span;

        unpack_kwargs!(
            kwargs,
            ndim: u8,
            (build, build_span): Arc<FnValue>,
        );

        let id = meta.id.clone();
        let name = meta.name.clone();
        let product_twist_system_build_fn = move |build_ctx: &BuildCtx| {
            let logger = &build_ctx.catalog.logger;

            build_ctx.set_building::<TwistSystem>();

            let mut scope = Scope::default();
            scope.special.id = Some(id.to_string().into());
            scope.special.ndim = Some(ndim);
            init_twists_in_hps_scope(&mut scope);
            let scope = Arc::new(scope);

            let build_fn = Arc::clone(&build);

            let id = id.clone();
            let name = name.clone();
            eval_tx.eval_blocking(move |runtime| {
                // IIFE to mimic try_block
                (|| {
                    let mut _exports = None;
                    let mut ctx = EvalCtx::new(&scope, runtime, caller_span, &mut _exports);

                    build_fn.call(build_span, &mut ctx, vec![], Map::new())?;

                    twists_builder_from_hps(&mut ctx, id, name)
                })()
                .map_err(|e| runtime.report_and_convert_to_eyre(e))
                .wrap_err("error building puzzle")
            })
        };

        Ok(Arc::new(TwistSystemProductBuildFn(Box::new(
            product_twist_system_build_fn,
        )))
        .into_generator_output(Arc::new(meta)))
    }
}

pub struct TwistSystemProductBuildFn(
    pub Box<dyn Send + Sync + for<'a> Fn(&'a BuildCtx) -> eyre::Result<TwistSystemProduct>>,
);

impl Component<GeneratorOutput<TwistSystem>> for TwistSystemProductBuildFn {}

impl TwistSystemProductBuildFn {
    pub fn into_generator_output(
        self: Arc<Self>,
        meta: Arc<CatalogMetadata>,
    ) -> GeneratorOutput<TwistSystem> {
        let build = self.build_fn();
        let mut components = ComponentList::new();
        components.insert(self);
        GeneratorOutput {
            meta,
            components,
            build,
        }
    }

    fn build_fn(self: &Arc<Self>) -> BuildFn<TwistSystem> {
        let this = Arc::clone(self);
        Arc::new(move |build_ctx| {
            let mut warn_fn = |e| build_ctx.catalog.logger.warn(format!("{e:?}"));
            this.0(&build_ctx)?.build(&build_ctx, &mut warn_fn)
        })
    }
}

pub(super) fn init_twists_in_hps_scope(scope: &mut Scope) {
    let mut m = Map::new();
    m.insert("points".into(), super::new_hps_list());
    m.insert("stabilizer_sets".into(), super::new_hps_list());
    m.insert("axes".into(), super::new_hps_list());
    m.insert("stabilizer_twists".into(), super::new_hps_list());
    scope.special.twists = Arc::new(Mutex::new(ValueData::Map(Arc::new(m)).at(BUILTIN_SPAN)));
}

pub(super) fn twists_builder_from_hps(
    ctx: &mut EvalCtx<'_>,
    id: CatalogId,
    name: String,
) -> Result<TwistSystemProduct> {
    let caller_span = ctx.caller_span;

    let mut twists_map = Arc::unwrap_or_clone(
        std::mem::take(&mut *ctx.scope.special.twists.lock()).to::<Arc<Map>>()?,
    );

    let (sym, sym_span) =
        pop_map_key::<Spanned<HpsSymmetry>>(&mut twists_map, BUILTIN_SPAN, "sym")?;
    let coxeter_matrix = sym.as_coxeter(sym_span)?.clone();
    let generators = coxeter_matrix
        .generator_motors()
        .at(sym_span)?
        .map(|g, m| (GenSeq::new([g]), m));

    let named_point_orbits: Vec<NamedPointOrbitSpec> =
        pop_map_key::<Vec<Value>>(&mut twists_map, BUILTIN_SPAN, "points")?
            .into_iter()
            .map(|value| super::named_orbit_from_value(ctx, &generators, value))
            .map_ok(|named_point_vectors| NamedPointOrbitSpec {
                named_point_vectors,
            })
            .try_collect()?;

    let mut axis_orbits: Vec<AxisOrbitSpec> =
        pop_map_key::<Vec<Value>>(&mut twists_map, BUILTIN_SPAN, "axes")?
            .into_iter()
            .map(|value| super::named_orbit_from_value(ctx, &generators, value))
            .map_ok(|named_axis_vectors| AxisOrbitSpec {
                named_axis_vectors,
                stabilizer_sets: vec![], // will be added later
            })
            .try_collect()?;

    for elem in pop_map_key::<List>(&mut twists_map, BUILTIN_SPAN, "stabilizer_twists")? {
        let [names_value, gizmo_pole_distance_value] = elem.to_array()?;
        let NonEmptyVec::<Spanned<String>>(mut names) = names_value.to()?;
        let (first_name, first_name_span) = names.remove(0); // always succeeds because nonempty
        let Some(orbit) = axis_orbits
            .iter_mut()
            .find(|o| o.contains_name(&first_name))
        else {
            ctx.warn_at(first_name_span, format!("no axis named {first_name:?}"));
            continue;
        };
        let names = names.into_iter().map(|(s, _)| s.into()).collect();
        let gizmo_pole_distance = gizmo_pole_distance_value.to()?;
        orbit.stabilizer_sets.push((names, gizmo_pole_distance));
    }

    let mut named_point_set_orbits = vec![];
    for elem in pop_map_key::<List>(&mut twists_map, BUILTIN_SPAN, "stabilizer_sets")? {
        let [names_value, gizmo_pole_distance_value] = elem.to_array()?;
        let NonEmptyVec::<Spanned<String>>(mut names) = names_value.to()?;
        let names = names.into_iter().map(|(s, _)| s.into()).collect();
        let gizmo_pole_distance = gizmo_pole_distance_value.to()?;
        named_point_set_orbits.push((names, gizmo_pole_distance));
    }

    let group = coxeter_matrix
        .isometry_group()
        .wrap_err("error expanding twist symmetries")
        .at(sym_span)?;
    // Shuffling group generators improves average word length, making some
    // group operations faster.
    let shuffled_group = crate::shuffle_group_generators(&group, &mut rand::rng())
        .wrap_err("error shuffling twist symmetry generators")
        .at(sym_span)?;

    TwistSystemProduct::new_factor(
        id,
        name,
        coxeter_matrix,
        shuffled_group,
        &axis_orbits,
        &named_point_orbits,
        &named_point_set_orbits,
        &mut ctx.warnf(),
    )
    .wrap_err("error building symmetric twist system")
    .at(caller_span)
}
