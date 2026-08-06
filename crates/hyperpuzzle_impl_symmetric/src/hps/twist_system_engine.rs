use std::sync::Arc;

use eyre::Context;
use hypergroup::GenSeq;
use hypermath::Vector;
use hyperpuzzle_core::{CatalogId, TwistSystem};
use hyperpuzzle_impl_nd_euclid::hps::HpsSymmetry;
use hyperpuzzlescript::{
    BUILTIN_SPAN, ErrorExt, EvalCtx, FnValue, HpsEngine, List, Map, NonEmptyVec, Result, Scope,
    Span, Spanned, SpecialVar, Type, Value, ValueData,
    engine::HpsEngineError,
    pop_kwarg, unpack_kwargs,
    util::{expect_end_of_map, pop_map_key, pop_map_key_in_special_var},
};
use hypuz_notation::Str;
use itertools::Itertools;
use parking_lot::Mutex;

use crate::{AxisOrbitSpec, NamedPointSetOrbitSpec, SimpleOrbitSpec};
use crate::{StabilizerTwistOrbitSpec, builder::*};

pub struct SymmetricTwistSystemEngine;

impl HpsEngine for SymmetricTwistSystemEngine {
    fn add_catalog_entries(
        &self,
        catalog: &hyperpuzzle_core::prelude::CatalogBuilder,
        eval_tx: &hyperpuzzlescript::EvalRequestTx,
        ctx: &mut EvalCtx<'_>,
        mut hps_gen: hyperpuzzlescript::engine::HpsGenerator,
    ) -> Result<(), HpsEngineError> {
        catalog.add::<TwistSystemProduct>(hps_gen.make_generator(
            eval_tx,
            move |build_ctx, tx, kwargs| {
                unpack_kwargs!(
                    kwargs,
                    ndim: u8,
                    (build, build_span): Arc<FnValue>,
                );

                let mut scope = Scope::default();
                scope.special.id = Some(build_ctx.id().to_string().into());
                scope.special.ndim = Some(ndim);
                init_twists_in_hps_scope(&mut scope);
                Ok(Arc::new(tx.eval_blocking(Arc::new(scope), move |ctx| {
                    build.call(build_span, ctx, vec![], Map::new())?;
                    twist_system_product_from_hps(ctx, build_span, build_ctx.id().clone())
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
    m.insert("stabilizer_sets".into(), super::new_hps_list());
    m.insert("axes".into(), super::new_hps_list());
    m.insert("stabilizer_twists".into(), super::new_hps_list());
    scope.special.twists = Arc::new(Mutex::new(ValueData::Map(Arc::new(m)).at(BUILTIN_SPAN)));
}

pub(super) fn twist_system_product_from_hps(
    ctx: &mut EvalCtx<'_>,
    build_span: Span,
    id: CatalogId,
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

    let named_point_orbits: Vec<SimpleOrbitSpec> = pop_map_key_in_special_var::<Vec<Value>>(
        &mut twists_map,
        build_span,
        SpecialVar::Twists,
        "points",
    )?
    .into_iter()
    .map(|value| super::named_orbit_from_value(ctx, &generators, value))
    .try_collect()?;

    let axis_orbits: Vec<AxisOrbitSpec> = pop_map_key_in_special_var::<Vec<Value>>(
        &mut twists_map,
        build_span,
        SpecialVar::Twists,
        "axes",
    )?
    .into_iter()
    .map(|value| {
        if value.is::<Vector>() {
            let vector = value.to::<Vector>()?;
            let prefix = Str::new();
            Ok(AxisOrbitSpec { prefix, vector })
        } else if value.is::<Map>() {
            let mut map = value.as_ref::<Map>()?.clone();
            let vector = pop_map_key(&mut map, value.span, "vector")?;
            let prefix = pop_map_key(&mut map, value.span, "prefix")?;
            expect_end_of_map(map, value.span)?;
            Ok(AxisOrbitSpec { prefix, vector })
        } else {
            Err(value.type_error(Type::Vec | Type::Map))
        }
    })
    .try_collect()?;

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
        &crate::FactorTwistSystemSpec {
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
    .at(caller_span)
}
