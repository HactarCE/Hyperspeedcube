//! Rhai `add_twist_system()` function.

use std::sync::Arc;

use hypermath::{ApproxHashMapKey, IndexNewtype, TransformByMotor, Vector, pga};
use hyperpuzzle_core::catalog::{BuildCtx, BuildTask, TwistSystemSpec};
use hyperpuzzle_impl_nd_euclid::builder::*;
use hyperpuzzle_impl_nd_euclid::{PerReferenceVector, ReferenceVector};
use itertools::Itertools;
use parking_lot::{MappedMutexGuard, Mutex, MutexGuard};
use rhai::Array;

use super::axis_system::RhaiAxisSystem;
use super::puzzle::RhaiPuzzle;
use super::*;
use crate::package::types::elements::{LockAs, RhaiAxis, RhaiTwist};
use crate::package::types::name_strategy::RhaiNameStrategy;
use crate::package::types::symmetry::RhaiSymmetry;

pub fn init_engine(engine: &mut Engine) {
    engine.register_type_with_name::<RhaiTwistSystem>("twistsystem");
}

pub fn register(module: &mut Module, catalog: &Catalog, eval_tx: &RhaiEvalRequestTx) {
    let cat = catalog.clone();
    let tx = eval_tx.clone();
    new_fn("add_twist_system").set_into_module(module, move |ctx: Ctx<'_>, map: Map| -> Result {
        let spec = twist_system_spec_from_rhai_map(&ctx, tx.clone(), map)?;
        cat.add_twist_system(Arc::new(spec)).eyrefmt()
    });

    let cat = catalog.clone();
    let tx = eval_tx.clone();
    new_fn("add_twist_system_generator").set_into_module(
        module,
        move |ctx: Ctx<'_>, gen_map: Map| -> Result {
            let tx = tx.clone();
            let generator = super::generator::generator_from_rhai_map::<TwistSystemSpec>(
                &ctx,
                tx.clone(),
                gen_map,
                move |ctx, build_ctx, map| {
                    build_ctx.progress.lock().task = BuildTask::BuildingTwists;
                    twist_system_spec_from_rhai_map(&ctx, tx.clone(), map)
                },
            )?;
            cat.add_twist_system_generator(Arc::new(generator))
                .eyrefmt()
        },
    );

    FuncRegistration::new_getter("ndim").set_into_module(
        module,
        |twist_system: &mut RhaiTwistSystem| -> Result<i64> {
            Ok(twist_system.lock()?.axes.ndim.into())
        },
    );

    FuncRegistration::new_index_getter().set_into_module(
        module,
        |twist_system: &mut RhaiTwistSystem, name: String| -> Result<RhaiTwist> {
            let opt_id = twist_system.lock()?.names.id_from_string(&name);
            Ok(RhaiTwist {
                id: opt_id.ok_or_else(|| format!("no twist named {name:?}"))?,
                db: Arc::new(twist_system.clone()),
            })
        },
    );

    FuncRegistration::new_getter("axes").set_into_module(
        module,
        |twist_system: &mut RhaiTwistSystem| -> RhaiAxisSystem {
            RhaiAxisSystem(twist_system.clone())
        },
    );

    new_fn("add_axis").set_into_module(
        module,
        |ctx: Ctx<'_>, twist_system: &mut RhaiTwistSystem, vector: Vector| -> Result<Dynamic> {
            twist_system.add_axes(&ctx, vector, None)
        },
    );

    new_fn("add_axis").set_into_module(
        module,
        |ctx: Ctx<'_>,
         twist_system: &mut RhaiTwistSystem,
         vector: Vector,
         names: Dynamic|
         -> Result<Dynamic> { twist_system.add_axes(&ctx, vector, Some(names)) },
    );

    new_fn("add_twist").set_into_module(
        module,
        |ctx: Ctx<'_>,
         twist_system: &mut RhaiTwistSystem,
         axis: RhaiAxis,
         transform: pga::Motor|
         -> Result<Dynamic> { twist_system.add_twists(&ctx, axis, transform, None) },
    );

    new_fn("add_twist").set_into_module(
        module,
        |ctx: Ctx<'_>,
         twist_system: &mut RhaiTwistSystem,
         axis: RhaiAxis,
         transform: pga::Motor,
         data: Map|
         -> Result<Dynamic> { twist_system.add_twists(&ctx, axis, transform, Some(data)) },
    );

    new_fn("add_twist_direction").set_into_module(
        module,
        |ctx: Ctx<'_>,
         twist_system: &mut RhaiTwistSystem,
         name: String,
         gen_twist: FnPtr|
         -> Result {
            if RhaiState::get(&ctx).lock().symmetry.is_some() {
                return Err("twist directions cannot use symmetry".into());
            }
            let twist_system_guard = twist_system.lock()?;
            if twist_system_guard.directions.contains_key(&name) {
                warn(&ctx, format!("duplicate twist direction name {name:?}"))?;
                return Ok(());
            }
            let axis_count = twist_system_guard.axes.len();
            drop(twist_system_guard);
            let db: Arc<dyn LockAs<AxisSystemBuilder>> = Arc::new(twist_system.clone());
            let mut twist_seqs = PerAxis::new();
            for id in Axis::iter(axis_count) {
                let axis = RhaiAxis {
                    id,
                    db: Arc::clone(&db),
                };
                let mut this = Dynamic::from(twist_system.clone());
                let twist_seq = from_rhai::<OptVecOrSingle<RhaiTwist>>(
                    &ctx,
                    gen_twist.call_raw(&ctx, Some(&mut this), &mut [Dynamic::from(axis)])?,
                )?
                .into_vec();

                twist_seqs
                    .push(if twist_seq.is_empty() {
                        None
                    } else {
                        Some(twist_seq.into_iter().map(|twist| twist.id).collect())
                    })
                    .map_err(|e| e.to_string())?;
            }
            twist_system.lock()?.directions.insert(name, twist_seqs);
            Ok(())
        },
    );

    new_fn("add_vantage_group").set_into_module(
        module,
        |ctx: Ctx<'_>, twist_system: &mut RhaiTwistSystem, data: Map| -> Result {
            let_from_map!(&ctx, data, {
                let id: String;
                let symmetry: RhaiSymmetry;
                let refs: Array;
                let init: Vec<String>;
            });

            match twist_system.lock()?.vantage_groups.entry(id.clone()) {
                indexmap::map::Entry::Occupied(_) => {
                    Err(format!("vantage group already exists with ID {id:?}").into())
                }
                indexmap::map::Entry::Vacant(e) => {
                    let mut reference_vectors = PerReferenceVector::new();
                    let mut reference_vector_names = NameSpecBiMapBuilder::new();
                    for pair in refs {
                        let [init, names] = from_rhai(&ctx, pair).in_key("refs")?;
                        // TODO: support types other than just vectors
                        // (anything that can be orbited? maybe just blades?)
                        let init_vector: Vector = from_rhai(&ctx, init).in_key("refs")?;
                        for (_gen_seq, _motor, vector, name) in symmetry
                            .orbit_with_names::<ReferenceVector, Vector>(
                                &ctx,
                                init_vector,
                                &from_rhai(&ctx, names)?,
                            )?
                        {
                            let id = reference_vectors.push(vector).map_err(|e| e.to_string())?;
                            reference_vector_names
                                .set(id, name)
                                .map_err(|e| e.to_string())?;
                        }
                    }

                    let preferred_reference_vectors = init
                        .into_iter()
                        .map(|s| {
                            reference_vector_names
                                .id_from_string(&s)
                                .ok_or(format!("no reference vector named {s:?}"))
                        })
                        .try_collect()?;

                    e.insert(VantageGroupBuilder {
                        symmetry: symmetry.isometry_group()?,
                        reference_vectors,
                        reference_vector_names,
                        preferred_reference_vectors,
                    });
                    Ok(())
                }
            }
        },
    );

    new_fn("add_vantage_set").set_into_module(
        module,
        |ctx: Ctx<'_>, twist_system: &mut RhaiTwistSystem, data: Map| -> Result {
            let_from_map!(&ctx, data, {
                let name: String;
                let group: String;
                let view_offset: Option<pga::Motor>;
                let transforms: Option<Map>;
                let axes: Option<Dynamic>;
                let directions: Option<Map>;
                let inherit_directions: Option<FnPtr>;
            });

            let ndim = twist_system.lock()?.axes.ndim;
            let ident = pga::Motor::ident(ndim);
            let view_offset = view_offset.unwrap_or_else(|| ident.clone());

            let transforms = transforms
                .unwrap_or_default()
                .into_iter()
                .map(|(k, v)| Ok((k.into(), from_rhai::<pga::Motor>(&ctx, v)?)))
                .collect::<Result<Vec<_>>>()?;

            let get_inherit_directions = |axis_vector: Vector| -> Result<Option<pga::Motor>> {
                let Some(f) = &inherit_directions else {
                    return Ok(None);
                };
                let ret: Dynamic = f.call_within_context(&ctx, [axis_vector])?;
                if ret.is_unit() {
                    Ok(None)
                } else {
                    Ok(Some(from_rhai::<pga::Motor>(&ctx, ret)?))
                }
            };

            // TODO: refactor and add more ways to specify relative axes & twists

            let mut axes = if axes.as_ref().is_some_and(|a| a.is_string()) {
                if axes.unwrap_or_default().into_string().as_deref() != Ok("*") {
                    return Err("invalid string for key `axes`; only \"*\" is allowed".into());
                }
                let axis_count = twist_system.lock()?.axes.len();
                Axis::iter(axis_count)
                    .map(|axis| -> Result<Option<(String, RelativeAxisBuilder)>> {
                        let twist_system_guard = twist_system.lock()?;
                        let Some(axis_name) = twist_system_guard.axes.names.get(axis) else {
                            return Ok(None);
                        };

                        let Ok(axis_info) = twist_system_guard.axes.get(axis) else {
                            return Ok(None);
                        };
                        let axis_name_spec = axis_name.spec.clone();
                        let axis_vector = axis_info.vector().clone();
                        drop(twist_system_guard); // Drop before running `get_inherit_directins()`

                        Ok(Some((
                            axis_name_spec,
                            RelativeAxisBuilder {
                                absolute_axis: axis,
                                transform: ident.clone(),
                                direction_map: AxisDirectionMapBuilder {
                                    directions: vec![],
                                    inherit: get_inherit_directions(axis_vector)?,
                                },
                            },
                        )))
                    })
                    .filter_map(Result::transpose)
                    .try_collect()?
            } else if let Some(v) = axes {
                // TODO: type error here does not explain that "*" is also allowed
                from_rhai::<Map>(&ctx, v)
                    .in_key("axes")?
                    .into_iter()
                    .map(|(k, pair)| {
                        let [transform, axis] = if pair.is_array() {
                            from_rhai::<[Dynamic; 2]>(&ctx, pair).in_key("axes")?
                        } else {
                            [Dynamic::from(pga::Motor::ident(ndim)), pair]
                        };

                        let absolute_axis = from_rhai::<RhaiAxis>(&ctx, axis).in_key("axes")?.id;
                        let transform = from_rhai::<pga::Motor>(&ctx, transform).in_key("axes")?;

                        let axis_vector = transform.transform(
                            twist_system
                                .lock()?
                                .axes
                                .get(absolute_axis)
                                .map_err(|e| e.to_string())?
                                .vector(),
                        );

                        Result::Ok((
                            k.into(),
                            RelativeAxisBuilder {
                                absolute_axis,
                                transform,
                                direction_map: AxisDirectionMapBuilder {
                                    directions: vec![],
                                    inherit: get_inherit_directions(axis_vector)?,
                                },
                            },
                        ))
                    })
                    .collect::<Result<Vec<_>>>()?
            } else {
                vec![]
            };

            for (k, v) in directions.unwrap_or_default() {
                let axis_name_key = format!("{k:?}");
                match axes
                    .iter_mut()
                    .find(|(name_spec, _)| hyperpuzzle_core::name_spec_matches_name(name_spec, &k))
                {
                    Some((_, relative_axis_builder)) => {
                        for (direction_name, pair) in from_rhai::<Map>(&ctx, v)
                            .in_key("directions")
                            .in_key(&axis_name_key)?
                        {
                            let direction_name_key = format!("{direction_name:?}");

                            let [transform, twist] = if pair.is_array() {
                                from_rhai::<[Dynamic; 2]>(&ctx, pair)
                                    .in_key("directions")
                                    .in_key(&axis_name_key)
                                    .in_key(&direction_name_key)?
                            } else {
                                [Dynamic::from(pga::Motor::ident(ndim)), pair]
                            };

                            let absolute_twist = from_rhai::<RhaiTwist>(&ctx, twist)
                                .in_key("directions")
                                .in_key(&axis_name_key)
                                .in_key(&direction_name_key)?
                                .id;
                            let transform = from_rhai::<pga::Motor>(&ctx, transform)
                                .in_key("directions")
                                .in_key(&axis_name_key)
                                .in_key(&direction_name_key)?;

                            relative_axis_builder.direction_map.directions.push((
                                direction_name.to_string(),
                                RelativeTwistBuilder {
                                    absolute_twist,
                                    transform,
                                },
                            ))
                        }
                    }
                    None => warn(&ctx, format!("no axis named {k:?}"))?,
                }
            }

            twist_system.lock()?.vantage_sets.push(VantageSetBuilder {
                name,
                group,
                view_offset,
                transforms,
                axes,
            });

            Ok(())
        },
    );
}

impl RhaiTwistSystem {
    fn add_axes(&self, ctx: &Ctx<'_>, vector: Vector, names: Option<Dynamic>) -> Result<Dynamic> {
        let mut twist_system = self.lock()?;
        let axis_system = &mut twist_system.axes;

        let rhai_state = RhaiState::get(ctx);
        let rhai_state_guard = rhai_state.lock();
        if let Some(symmetry) = &rhai_state_guard.symmetry {
            let mut first_axis = None;
            let mut orbit_elements = vec![];
            let mut generator_sequences = vec![];

            // Add the axes.
            let names = from_rhai_opt(ctx, names)?;
            for (gen_seq, _motor, v, name) in
                symmetry.orbit_with_names::<Axis, Vector>(ctx, vector.clone(), &names)?
            {
                let axis = self.add_axis(axis_system, v, name)?;
                orbit_elements.push(Some(axis.id));
                generator_sequences.push(gen_seq);
                first_axis.get_or_insert(axis);
            }

            // Add the orbit.
            axis_system.orbits.push(Orbit {
                elements: Arc::new(orbit_elements),
                generator_sequences: Arc::new(generator_sequences),
            });

            Ok(Dynamic::from(first_axis.ok_or("no axes added")?))
        } else {
            let axis = self.add_axis(axis_system, vector, from_rhai_opt(ctx, names)?)?;
            Ok(Dynamic::from(axis))
        }
    }

    // TODO: document that this promises not to deadlock
    fn add_axis(
        &self,
        axis_system: &mut AxisSystemBuilder,
        vector: Vector,
        name: Option<String>,
    ) -> Result<RhaiAxis> {
        // Add the axis.
        let id = axis_system.add(vector).eyrefmt()?;

        // Assign name.
        axis_system.names.set(id, name).map_err(|e| e.to_string())?;

        Ok(RhaiAxis {
            id,
            db: Arc::new(RhaiAxisSystem(self.clone())),
        })
    }

    fn add_twists(
        &self,
        ctx: &Ctx<'_>,
        init_axis: RhaiAxis,
        init_transform: pga::Motor,
        data: Option<Map>,
    ) -> Result<Dynamic> {
        let twist_system = self.lock()?;
        let ndim = twist_system.axes.ndim;

        let_from_map!(ctx, data.unwrap_or_default(), {
            let inverse: Option<bool>;
            let multipliers: Option<bool>;
            let prefix: Option<RhaiNameStrategy>;
            let name: Option<RhaiNameStrategy>;
            let suffix: Option<RhaiNameStrategy>;
            let qtm: Option<usize>;
            let gizmo_pole_distance: Option<f32>;
        });

        // Should we generate inverses? Default true only in 3D.
        let inverse = inverse.unwrap_or(ndim == 3);
        // Should we generate multipliers? Default true only in 3D.
        let multipliers = multipliers.unwrap_or(ndim == 3);

        let sym = RhaiState::get(ctx).lock().symmetry.clone();

        let init_axis_vector = twist_system
            .axes
            .get(init_axis.id)
            .map_err(|e| e.to_string())?
            .vector()
            .clone();

        let init = AxisAndTransform::new(init_axis_vector, init_transform)?;

        let do_naming;
        {
            let has_prefix = prefix.as_ref().is_none_or(|s| !s.is_empty()); // prefix defaults to axis
            let has_name = name.as_ref().is_some_and(|s| !s.is_empty());
            let has_suffix = suffix.as_ref().is_some_and(|s| !s.is_empty());
            do_naming = has_prefix || has_name || has_suffix;
        }

        let prefix_fn = prefix
            .map(|s| s.name_fn::<Twist, _>(ctx, sym.as_ref(), &init))
            .transpose()?;

        let name_fn = name
            .map(|s| s.name_fn::<Twist, _>(ctx, sym.as_ref(), &init))
            .transpose()?;

        let suffix_fn = suffix
            .map(|s| s.name_fn::<Twist, _>(ctx, sym.as_ref(), &init))
            .transpose()?;

        let qtm = qtm.unwrap_or(1);
        if qtm < 1 {
            warn(ctx, "twist has QTM value less than 1")?;
        }

        if gizmo_pole_distance.is_some() && ndim != 3 && ndim != 4 {
            return Err("twist gizmo is only supported in 3D and 4D".into());
        }

        let get_name = |motor: &pga::Motor, obj: &AxisAndTransform, i: i64| {
            if do_naming {
                let args = vec![Dynamic::from(motor.clone()), Dynamic::from(i)];

                // `prefix` defaults to axis name.
                let prefix = match &prefix_fn {
                    Some(f) => f.call_with_args(ctx, obj, args.clone())?,
                    None => {
                        let db = self.lock()?;
                        // IIFE to mimic try_block
                        (|| {
                            let axis_id = db.axes.vector_to_id(&obj.ax_vec)?;
                            let axis_name = db.axes.names.get(axis_id)?;
                            Some(axis_name.spec.clone())
                        })()
                    }
                };

                // `name` defaults to empty.
                let name = match &name_fn {
                    Some(f) => f.call_with_args(ctx, obj, args.clone())?,
                    None => None,
                };

                // `suffix` defaults to number + `'`.
                let suffix = match &suffix_fn {
                    Some(f) => f.call_with_args(ctx, obj, args)?,
                    None => {
                        let inverse = if i < 0 { "'" } else { "" };
                        if i.abs() > 1 {
                            Some(format!("{}{inverse}", i.abs().to_string()))
                        } else {
                            Some(inverse.to_string())
                        }
                    }
                };

                let ret = prefix.unwrap_or_default()
                    + name.unwrap_or_default().as_str()
                    + suffix.unwrap_or_default().as_str();

                if ret.contains("|") {
                    void_warn(ctx)(format!(
                        "name {ret:?} formed by concatenation may be incorrect"
                    ));
                }
                Ok(Some(ret).filter(|s| !s.is_empty()))
            } else {
                Ok(None)
            }
        };

        drop(twist_system); // Unlock mutex

        if let Some(symmetry) = sym {
            let mut first_twist = None;

            // Add the twists.
            for (_gen_seq, motor, obj) in symmetry.orbit(init) {
                let twist = self.add_twist(
                    ctx,
                    &obj,
                    qtm,
                    gizmo_pole_distance,
                    inverse,
                    multipliers,
                    |i| get_name(&motor, &obj, i),
                )?;
                first_twist.get_or_insert(twist);
            }

            Ok(Dynamic::from(first_twist.ok_or("no twists added")?))
        } else {
            let ident = pga::Motor::ident(ndim);
            let twist = self.add_twist(
                ctx,
                &init,
                qtm,
                gizmo_pole_distance,
                inverse,
                multipliers,
                |i| get_name(&ident, &init, i),
            )?;
            Ok(Dynamic::from(twist.ok_or("no twists added")?))
        }
    }

    fn add_twist(
        &self,
        ctx: &Ctx<'_>,
        axis_and_transform: &AxisAndTransform,
        qtm: usize,
        gizmo_pole_distance: Option<f32>,
        inverse: bool,
        multipliers: bool,
        get_name: impl Fn(i64) -> Result<Option<String>>,
    ) -> Result<Option<RhaiTwist>> {
        let ndim = self.lock()?.axes.ndim;

        let axis_vector = &axis_and_transform.ax_vec;
        let base_transform = &axis_and_transform.transform;

        let Some(axis) = self.lock()?.axes.vector_to_id(axis_vector) else {
            return Ok(None);
        };
        let transform = base_transform.clone();
        let name = get_name(1)?;
        let Some(first_twist_id) = self
            .lock()?
            .add_named(
                TwistBuilder {
                    axis,
                    transform,
                    qtm,
                    gizmo_pole_distance,
                    include_in_scrambles: true,
                },
                name,
                void_warn(ctx),
            )
            .eyrefmt()?
        else {
            return Ok(None);
        };
        if inverse {
            let transform = base_transform.reverse();
            let is_equivalent_to_reverse = base_transform.is_self_reverse();
            let name = get_name(-1)?;
            self.lock()?
                .add_named(
                    TwistBuilder {
                        axis,
                        transform,
                        qtm,
                        gizmo_pole_distance: gizmo_pole_distance.filter(|_| ndim > 3),
                        include_in_scrambles: !is_equivalent_to_reverse,
                    },
                    name,
                    void_warn(ctx),
                )
                .eyrefmt()?;
        }

        let mut previous_transform = base_transform.clone();
        for i in 2.. {
            if !multipliers {
                break;
            }

            // Check whether we've exceeded the max repeat count.
            if i > crate::MAX_TWIST_REPEAT as i64 {
                return Err(format!(
                    "twist transform takes too long to repeat! exceeded maximum of {}",
                    crate::MAX_TWIST_REPEAT,
                )
                .into());
            }

            let transform = &previous_transform * base_transform;

            // Check whether we've reached the inverse.
            if inverse {
                if previous_transform.is_self_reverse()
                    || transform.is_equivalent_to(&previous_transform.reverse())
                {
                    break;
                }
            } else if transform.is_ident() {
                break;
            }
            previous_transform = transform.clone();

            let name = get_name(i)?;
            self.lock()?
                .add_named(
                    TwistBuilder {
                        axis,
                        transform,
                        qtm: qtm * i as usize,
                        gizmo_pole_distance: None, // no gizmo for multiples
                        include_in_scrambles: true,
                    },
                    name,
                    void_warn(ctx),
                )
                .eyrefmt()?;

            if inverse {
                let transform = previous_transform.reverse();
                let is_equivalent_to_reverse = previous_transform.is_self_reverse();
                let name = get_name(-i)?;
                self.lock()?
                    .add_named(
                        TwistBuilder {
                            axis,
                            transform,
                            qtm: qtm * i as usize,
                            gizmo_pole_distance: None, // no gizmo for multiples
                            include_in_scrambles: !is_equivalent_to_reverse,
                        },
                        name,
                        void_warn(ctx),
                    )
                    .eyrefmt()?;
            }
        }

        Ok(Some(RhaiTwist {
            id: first_twist_id,
            db: Arc::new(self.clone()),
        }))
    }
}

/// Constructs a twist system spec from a Rhai specification.
pub fn twist_system_spec_from_rhai_map(
    ctx: &Ctx<'_>,
    eval_tx: RhaiEvalRequestTx,
    data: Map,
) -> Result<TwistSystemSpec> {
    let_from_map!(ctx, data, {
        let id: String;
        let name: Option<String>;
        let ndim: u8;
        let build: FnPtr;
    });

    let id_clone = id.clone();
    let name_clone = name.clone();
    let build_clone = build.clone();

    let create_twist_system_builder =
        move |_build_ctx: &BuildCtx| -> eyre::Result<RhaiTwistSystem> {
            let mut builder = TwistSystemBuilder::new_shared(id.clone(), ndim);
            builder.name = name.clone();

            Ok(RhaiTwistSystem::new(builder))
        };

    let build_from_twist_system_builder = crate::util::rhai_eval_fn(
        ctx,
        eval_tx,
        &build_clone,
        move |ctx: Ctx<'_>, (build_ctx, builder): (BuildCtx, RhaiTwistSystem)| {
            let mut this = Dynamic::from(builder.clone());

            let () = RhaiState::with_ndim(&ctx, ndim, |ctx| {
                Ok(from_rhai(ctx, build.call_raw(ctx, Some(&mut this), [])?)?)
            })?;

            builder.lock()?.is_modified = false; // this is not ad-hoc

            builder
                .lock()?
                .build(Some(&build_ctx), None, void_warn(&ctx))
                .map(|ok| Redirectable::Direct(Arc::new(ok)))
        },
    );

    Ok(TwistSystemSpec {
        id: id_clone.clone(),
        name: name_clone.unwrap_or(id_clone),
        build: Box::new(move |build_ctx| {
            let builder = create_twist_system_builder(&build_ctx)?;
            build_from_twist_system_builder((build_ctx, builder))?
        }),
    })
}

#[derive(Debug, Clone)]
pub enum RhaiTwistSystem {
    Puzzle(RhaiPuzzle),
    Twists(Arc<Mutex<TwistSystemBuilder>>),
}
impl RhaiTwistSystem {
    pub fn new(twist_system_builder: TwistSystemBuilder) -> Self {
        Self::Twists(Arc::new(Mutex::new(twist_system_builder)))
    }

    pub fn get(&self, twist_name: &str) -> Result<Option<RhaiTwist>> {
        self.lock().map(|builder| {
            builder
                .names
                .id_from_string(twist_name)
                .map(|id| RhaiTwist {
                    id,
                    db: Arc::new(self.clone()),
                })
        })
    }

    // TODO: THIS IS A REALLY NASTY HACK
    pub fn lock(&self) -> Result<MappedMutexGuard<'_, TwistSystemBuilder>> {
        <Self as LockAs<TwistSystemBuilder>>::lock(self)
    }
}
impl LockAs<TwistSystemBuilder> for RhaiTwistSystem {
    fn lock(&self) -> Result<MappedMutexGuard<'_, TwistSystemBuilder>> {
        match self {
            RhaiTwistSystem::Puzzle(puzzle) => LockAs::lock(puzzle),
            RhaiTwistSystem::Twists(twists) => {
                MutexGuard::try_map(twists.lock(), |contents| Some(contents))
                    .map_err(|_| "no twist system".into())
            }
        }
    }
}
impl LockAs<AxisSystemBuilder> for RhaiTwistSystem {
    fn lock(&self) -> Result<MappedMutexGuard<'_, AxisSystemBuilder>> {
        match self {
            RhaiTwistSystem::Puzzle(puzzle) => LockAs::lock(puzzle),
            RhaiTwistSystem::Twists(twists) => {
                MutexGuard::try_map(twists.lock(), |contents| Some(&mut contents.axes))
                    .map_err(|_| "no twist system".into())
            }
        }
    }
}

/// Axis and transform representing a twist to be created.
///
/// The orientation of `transform` is preserved when reflected. For example, a
/// 90-degree clockwise rotation in 2D will always be clockwise no matter how it
/// is transformed.
#[derive(Debug, Clone)]
struct AxisAndTransform {
    ax_vec: Vector,
    transform: pga::Motor,
}
impl AxisAndTransform {
    pub fn new(ax_vec: Vector, transform: pga::Motor) -> Result<Self> {
        let transform = transform
            .canonicalize_up_to_180()
            .ok_or("bad twist transform")?;
        Ok(Self { ax_vec, transform })
    }
}
impl TransformByMotor for AxisAndTransform {
    fn transform_by(&self, m: &pga::Motor) -> Self {
        let ax_vec = self.ax_vec.transform_by(m);

        // Preserve orientation of transform.
        let mut transform = self.transform.transform_by(m);
        if m.is_reflection() {
            transform = transform.reverse();
        }
        // Canonicalize transform for comparison in `ApproxHashMap`.
        transform = transform.canonicalize_up_to_180().unwrap_or(transform);

        Self { ax_vec, transform }
    }
}
impl ApproxHashMapKey for AxisAndTransform {
    type Hash = (
        <Vector as ApproxHashMapKey>::Hash,
        <pga::Motor as ApproxHashMapKey>::Hash,
    );

    fn approx_hash(
        &self,
        mut float_hash_fn: impl FnMut(
            hypermath::Float,
        ) -> hypermath::collections::approx_hashmap::FloatHash,
    ) -> Self::Hash {
        (
            self.ax_vec.approx_hash(&mut float_hash_fn),
            self.transform.approx_hash(float_hash_fn),
        )
    }
}
