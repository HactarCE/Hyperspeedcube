use std::sync::Arc;

use eyre::{Context, eyre};
use hypergroup::GenSeq;
use hyperpuzzle_core::{BuildCtx, TwistSystem};
use hyperpuzzle_impl_nd_euclid::hps::HpsSymmetry;
use hyperpuzzlescript::engine::HpsEngineError;
use hyperpuzzlescript::util::pop_map_key_in_special_var;
use hyperpuzzlescript::{
    BUILTIN_SPAN, ErrorExt, EvalCtx, FnValue, HpsEngine, List, Map, NonEmptyVec, Result, Scope,
    Span, Spanned, SpecialVar, Value, ValueData, unpack_kwargs,
};
use hypuz_notation::Str;
use itertools::Itertools;
use parking_lot::Mutex;

use crate::builder::*;
use crate::{NamedPointOrbitSpec, NamedPointSetOrbitSpec, StabilizerTwistOrbitSpec};

pub struct SymmetricTwistSystemEngine;

impl HpsEngine for SymmetricTwistSystemEngine {
    fn add_catalog_entries(
        &self,
        catalog: &hyperpuzzle_core::prelude::CatalogBuilder,
        eval_tx: &hyperpuzzlescript::EvalRequestTx,
        ctx: &mut EvalCtx<'_>,
        hps_gen: hyperpuzzlescript::engine::HpsGenerator,
    ) -> Result<(), HpsEngineError> {
        catalog.add::<TwistSystemProduct>(hps_gen.make_generator(
            eval_tx,
            move |build_ctx, tx, kwargs| {
                let id = build_ctx.id();
                unpack_kwargs!(
                    kwargs,
                    ndim: u8,
                    (build, build_span): Arc<FnValue>,
                    name: String = {
                        build_ctx.warn_fn()(eyre!("missing `name` for twist system `{id}`"));
                        id.to_string()
                    },
                );

                let mut scope = Scope::default();
                scope.special.id = Some(id.to_string().into());
                scope.special.ndim = Some(ndim);
                init_twists_in_hps_scope(&mut scope);
                Ok(Arc::new(tx.eval_blocking(Arc::new(scope), move |ctx| {
                    build.call(build_span, ctx, vec![], Map::new())?;
                    twist_system_product_from_hps(&build_ctx, ctx, build_span, name)
                })?))
            },
        ))?;

        catalog.add::<TwistSystem>(hps_gen.make_generator(
            eval_tx,
            move |build_ctx, _tx, _kwargs| {
                Ok(build_ctx
                    .build_blocking::<TwistSystemProduct>(build_ctx.id())?
                    .build(&build_ctx, &mut build_ctx.warn_fn())?)
            },
        ))?;

        Ok(())
    }
}

pub(super) fn init_twists_in_hps_scope(scope: &mut Scope) {
    let mut m = Map::new();
    m.insert("points".into(), super::new_hps_list());
    m.insert("axes".into(), super::new_hps_list());
    m.insert("stabilizer_twists".into(), super::new_hps_list());
    m.insert("stabilizer_sets".into(), super::new_hps_list());
    scope.special.twists = Arc::new(Mutex::new(ValueData::Map(Arc::new(m)).at(BUILTIN_SPAN)));
}

pub(super) fn twist_system_product_from_hps(
    build_ctx: &BuildCtx,
    ctx: &mut EvalCtx<'_>,
    build_span: Span,
    name: String,
) -> Result<TwistSystemProduct> {
    let caller_span = ctx.caller_span;

    let mut twists_map = Arc::unwrap_or_clone(
        std::mem::take(&mut *ctx.scope.special.twists.lock()).to::<Arc<Map>>()?,
    );

    let (sym, sym_span) = pop_map_key_in_special_var::<Spanned<HpsSymmetry>>(
        &mut twists_map,
        build_span,
        SpecialVar::Twists,
        "sym",
    )?;
    let coxeter_matrix = sym.as_coxeter(sym_span)?.clone();
    let generators = coxeter_matrix
        .generator_motors()
        .at(sym_span)?
        .map(|g, m| (GenSeq::new([g]), m));

    let mut autonames = crate::named_point_autonames();
    let named_point_orbits: Vec<NamedPointOrbitSpec> = pop_map_key_in_special_var::<Vec<Value>>(
        &mut twists_map,
        build_span,
        SpecialVar::Twists,
        "points",
    )?
    .into_iter()
    .map(|value| super::named_point_orbit_from_value(ctx, &generators, value, &mut autonames))
    .try_collect()?;

    let axis_orbits = super::simple_orbit_from_value(pop_map_key_in_special_var(
        &mut twists_map,
        build_span,
        SpecialVar::Twists,
        "axes",
    )?)?;

    let mut stabilizer_twist_orbits = vec![];
    for elem in pop_map_key_in_special_var::<List>(
        &mut twists_map,
        build_span,
        SpecialVar::Twists,
        "stabilizer_twists",
    )? {
        let [names_value, gizmo_pole_distance_value] = elem.to_array()?;
        let NonEmptyVec::<Str>(mut names) = names_value.to()?;
        stabilizer_twist_orbits.push(StabilizerTwistOrbitSpec {
            axis_name: names.remove(0), // always succeeds because nonempty
            named_points: names,
            gizmo_pole_distance: gizmo_pole_distance_value.to()?,
        });
    }

    let mut named_point_set_orbits = vec![];
    for elem in pop_map_key_in_special_var::<List>(
        &mut twists_map,
        build_span,
        SpecialVar::Twists,
        "stabilizer_sets",
    )? {
        let [names_value, gizmo_pole_distance_value] = elem.to_array()?;
        named_point_set_orbits.push(NamedPointSetOrbitSpec {
            named_points: names_value.to()?,
            gizmo_pole_distance: gizmo_pole_distance_value.to()?,
        });
    }

    build_ctx.push_task("Constructing twist system factor");
    let result = TwistSystemProduct::new_factor(
        &crate::FactorTwistSystemSpec {
            id: build_ctx.id().clone(),
            name,
            ndim: coxeter_matrix.generator_count(),
            coxeter_matrix: Some(coxeter_matrix),
            axis_orbits,
            named_point_orbits,
            named_point_set_orbits,
            stabilizer_twist_orbits,
        },
        &mut ctx.warnf(),
    )
    .wrap_err("error building symmetric twist system")
    .at(caller_span);
    build_ctx.pop_task();

    result
}
