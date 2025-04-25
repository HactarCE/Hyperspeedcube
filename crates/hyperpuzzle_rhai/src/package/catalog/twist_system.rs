//! Rhai `add_twist_system()` function.

use std::sync::Arc;

use eyre::eyre;
use hypermath::{ApproxHashMap, ApproxHashMapKey, IndexNewtype, TransformByMotor, Vector, pga};
use hyperpuzzle_core::catalog::{BuildCtx, BuildTask, TwistSystemSpec};
use hyperpuzzle_impl_nd_euclid::builder::*;
use parking_lot::{MappedMutexGuard, Mutex, MutexGuard};

use super::{axis_system::RhaiAxisSystem, *};
use crate::package::types::elements::{LockAs, RhaiAxis, RhaiTwist};

pub fn init_engine(engine: &mut Engine) {
    engine.register_type_with_name::<RhaiTwistSystemBuilder>("twistsystem");
}

pub fn register(module: &mut Module, catalog: &Catalog, eval_tx: &RhaiEvalRequestTx) {
    let cat = catalog.clone();
    let tx = eval_tx.clone();
    new_fn("add_twist_system").set_into_module(
        module,
        move |ctx: Ctx<'_>, map: Map| -> Result<()> {
            let spec = twist_system_spec_from_rhai_map(&ctx, tx.clone(), map)?;
            cat.add_twist_system(Arc::new(spec)).eyrefmt()
        },
    );

    let cat = catalog.clone();
    let tx = eval_tx.clone();
    new_fn("add_twist_system_generator").set_into_module(
        module,
        move |ctx: Ctx<'_>, gen_map: Map| -> Result<()> {
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
        |twist_system: &mut RhaiTwistSystemBuilder| -> Result<u8> {
            Ok(twist_system.lock()?.axes.ndim)
        },
    );

    FuncRegistration::new_index_getter().set_into_module(
        module,
        |twist_system: &mut RhaiTwistSystemBuilder, name: String| -> Result<RhaiTwist> {
            let opt_id = twist_system.lock()?.names.id_from_string(&name);
            Ok(RhaiTwist {
                id: opt_id.ok_or_else(|| format!("no twist named {name:?}"))?,
                db: Arc::new(twist_system.clone()),
            })
        },
    );

    FuncRegistration::new_getter("axes").set_into_module(
        module,
        |twist_system: &mut RhaiTwistSystemBuilder| -> RhaiAxisSystem {
            RhaiAxisSystem(twist_system.clone())
        },
    );

    new_fn("add_axis").set_into_module(
        module,
        move |ctx: Ctx<'_>,
              twist_system: RhaiTwistSystemBuilder,
              vector: Vector|
              -> Result<Dynamic> { twist_system.add_axes(&ctx, vector, None) },
    );

    new_fn("add_axis").set_into_module(
        module,
        move |ctx: Ctx<'_>,
              twist_system: RhaiTwistSystemBuilder,
              vector: Vector,
              names: Dynamic|
              -> Result<Dynamic> { twist_system.add_axes(&ctx, vector, Some(names)) },
    );

    new_fn("add_twist").set_into_module(
        module,
        move |ctx: Ctx<'_>,
              twist_system: RhaiTwistSystemBuilder,
              axis: RhaiAxis,
              transform: pga::Motor|
              -> Result<Dynamic> { twist_system.add_twists(&ctx, axis, transform, None) },
    );

    new_fn("add_twist").set_into_module(
        module,
        move |ctx: Ctx<'_>,
              twist_system: RhaiTwistSystemBuilder,
              axis: RhaiAxis,
              transform: pga::Motor,
              data: Map|
              -> Result<Dynamic> {
            twist_system.add_twists(&ctx, axis, transform, Some(data))
        },
    );

    new_fn("add_twist_direction").set_into_module(
        module,
        move |ctx: Ctx<'_>,
              twist_system: RhaiTwistSystemBuilder,
              name: String,
              gen_twist: FnPtr|
              -> Result<()> {
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
}

impl RhaiTwistSystemBuilder {
    fn add_axes(&self, ctx: &Ctx<'_>, vector: Vector, names: Option<Dynamic>) -> Result<Dynamic> {
        let mut twist_system = self.lock()?;
        let axis_system = &mut twist_system.axes;

        let rhai_state = RhaiState::get(ctx);
        let rhai_state_guard = rhai_state.lock();
        if let Some(symmetry) = &rhai_state_guard.symmetry {
            let mut first_axis = None;
            let mut orbit_elements = vec![];
            let mut generator_sequences = vec![];

            let mut vector_to_name = ApproxHashMap::new();
            if let Some(names) = names {
                let name_specs_and_gen_seqs =
                    orbit_names::names_from_table::<Axis>(ctx, from_rhai(ctx, names)?)?;
                for (name, gen_seq) in name_specs_and_gen_seqs {
                    let v = symmetry.motor_for_gen_seq(gen_seq)?.transform(&vector);
                    vector_to_name.insert(v, name.spec);
                }
            }

            // Add the axes.
            for (gen_seq, _motor, v) in symmetry.orbit(vector.clone()) {
                let name = vector_to_name.get(&v).cloned();
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
            let multipliers: Option<bool>;
            let inverse: Option<bool>;
            let prefix: Option<String>;
            let name: Option<String>;
            let suffix: Option<String>;
            let inv_name: Option<String>;
            let inv_suffix: Option<String>;
            let name_fn: Option<FnPtr>;
            let qtm: Option<usize>;
            let gizmo_pole_distance: Option<f32>;
        });

        // Prefix defaults to axis name.
        let get_prefix = |axis: Axis| match &prefix {
            Some(s) => Some(s.to_owned()),
            None => self
                .lock()
                .ok()?
                .axes
                .names
                .get(axis)
                .map(|name| name.spec.clone()),
        };

        let do_naming = prefix.as_ref().is_none_or(|s| !s.is_empty()) // prefix defaults to axis
            || name.as_ref().is_some_and(|s| !s.is_empty())
            || suffix.as_ref().is_some_and(|s| !s.is_empty())
            || inv_name.as_ref().is_some_and(|s| !s.is_empty())
            || inv_suffix.as_ref().is_some_and(|s| !s.is_empty())
            || name_fn.is_some();

        let inverse = inverse.unwrap_or(ndim == 3);
        let multipliers = multipliers.unwrap_or(ndim == 3);

        let suffix = suffix.unwrap_or_default();
        let inv_suffix = inv_suffix.unwrap_or_else(|| match &inv_name {
            Some(_) => suffix.clone(),
            None => "'".to_string(),
        });

        if name_fn.is_some() && (name.is_some() || inv_name.is_some()) {
            return Err(
                "when `name_fn` is specified, `name` and `inv_name` must not be specified".into(),
            );
        }

        let name = name.unwrap_or_default();
        let inv_name = inv_name.unwrap_or_else(|| name.clone());

        let qtm = qtm.unwrap_or(1);
        if qtm < 1 {
            warn(ctx, "twist has QTM value less than 1")?;
        }

        if gizmo_pole_distance.is_some() && ndim != 3 && ndim != 4 {
            return Err("twist gizmo is only supported in 3D and 4D".into());
        }

        let get_name = |axis: Axis, i: i32| {
            if let Some(name_fn) = &name_fn {
                name_fn.call_within_context(ctx, [i])
            } else if do_naming {
                if i == 0 {
                    return Err("bad twist multiplier".into());
                }
                let mut ret = get_prefix(axis).unwrap_or_default();
                ret += if i > 0 { &name } else { &inv_name };
                if i.abs() >= 2 {
                    ret += &i.abs().to_string();
                }
                ret += if i > 0 { &suffix } else { &inv_suffix };
                if ret.contains("|") {
                    void_warn(ctx)(format!(
                        "name {ret:?} formed by concatenation may be incorrect"
                    ));
                }
                Ok(ret)
            } else {
                Ok(String::new())
            }
        };

        let init_axis_vector = twist_system
            .axes
            .get(init_axis.id)
            .map_err(|e| e.to_string())?
            .vector()
            .clone();

        drop(twist_system); // Unlock mutex

        let rhai_state = RhaiState::get(ctx);
        let rhai_state_guard = rhai_state.lock();
        if let Some(symmetry) = &rhai_state_guard.symmetry {
            let mut first_twist = None;

            let init = AxisAndTransform {
                ax_vec: init_axis_vector,
                transform: init_transform,
            };

            // Add the twists.
            for (_gen_seq, _motor, AxisAndTransform { ax_vec, transform }) in symmetry.orbit(init) {
                let twist = self.add_twist(
                    ctx,
                    &ax_vec,
                    transform,
                    qtm,
                    gizmo_pole_distance,
                    inverse,
                    multipliers,
                    get_name,
                )?;
                first_twist.get_or_insert(twist);
            }

            Ok(Dynamic::from(first_twist.ok_or("no twists added")?))
        } else {
            let twist = self.add_twist(
                ctx,
                &init_axis_vector,
                init_transform,
                qtm,
                gizmo_pole_distance,
                inverse,
                multipliers,
                get_name,
            )?;
            Ok(Dynamic::from(twist.ok_or("no twists added")?))
        }
    }

    fn add_twist(
        &self,
        ctx: &Ctx<'_>,
        axis_vector: &Vector,
        base_transform: pga::Motor,
        qtm: usize,
        gizmo_pole_distance: Option<f32>,
        inverse: bool,
        multipliers: bool,
        get_name: impl Fn(Axis, i32) -> Result<String>,
    ) -> Result<Option<RhaiTwist>> {
        let ndim = self.lock()?.axes.ndim;

        let Some(axis) = self.lock()?.axes.vector_to_id(axis_vector) else {
            return Ok(None);
        };
        let transform = base_transform.clone();
        let name = get_name(axis, 1)?;
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
            let name = get_name(axis, -1)?;
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
            if i > crate::MAX_TWIST_REPEAT as i32 {
                return Err(format!(
                    "twist transform takes too long to repeat! exceeded maximum of {}",
                    crate::MAX_TWIST_REPEAT,
                )
                .into());
            }

            let transform = &previous_transform * &base_transform;

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

            let name = get_name(axis, i)?;
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
                let name = get_name(axis, -i)?;
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

/// Constructs a twist system from a Rhai specification.
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

    let build_fn = move |ctx: Ctx<'_>, build_ctx: BuildCtx| {
        let mut builder = TwistSystemBuilder::new_shared(id.clone(), ndim);
        builder.name = name.clone();

        let builder = RhaiTwistSystemBuilder::new(builder);

        let mut this = Dynamic::from(builder.clone());

        let () = RhaiState::with_ndim(&ctx, ndim, |ctx| {
            Ok(from_rhai(ctx, build.call_raw(ctx, Some(&mut this), [])?)?)
        })?;
        builder.lock()?.is_modified = false; // this is not ad-hoc

        builder
            .0
            .lock()
            .take()
            .ok_or_else(|| eyre!("twist system builder is missing"))?
            .build(Some(&build_ctx), None, void_warn(&ctx))
            .map(|ok| Redirectable::Direct(Arc::new(ok)))
    };

    let build = crate::util::rhai_eval_fn(ctx, eval_tx, &build_clone, build_fn);

    Ok(TwistSystemSpec {
        id: id_clone.clone(),
        name: name_clone.unwrap_or(id_clone),
        build: Box::new(move |build_ctx| build(build_ctx).and_then(|r| r)),
    })
}

#[derive(Debug, Clone)]
pub struct RhaiTwistSystemBuilder(pub Arc<Mutex<Option<TwistSystemBuilder>>>);
impl RhaiTwistSystemBuilder {
    pub fn new(twist_system_builder: TwistSystemBuilder) -> Self {
        Self(Arc::new(Mutex::new(Some(twist_system_builder))))
    }

    // TODO: THIS IS A REALLY NASTY HACK
    pub fn lock(&self) -> Result<MappedMutexGuard<'_, TwistSystemBuilder>> {
        <Self as LockAs<TwistSystemBuilder>>::lock(self)
    }
}
impl LockAs<TwistSystemBuilder> for RhaiTwistSystemBuilder {
    fn lock(&self) -> Result<MappedMutexGuard<'_, TwistSystemBuilder>> {
        MutexGuard::try_map(self.0.lock(), |contents| contents.as_mut())
            .map_err(|_| "no twist system".into())
    }
}
impl LockAs<AxisSystemBuilder> for RhaiTwistSystemBuilder {
    fn lock(&self) -> Result<MappedMutexGuard<'_, AxisSystemBuilder>> {
        MutexGuard::try_map(self.0.lock(), |contents| Some(&mut contents.as_mut()?.axes))
            .map_err(|_| "no twist system".into())
    }
}

#[derive(Debug, Clone)]
struct AxisAndTransform {
    ax_vec: Vector,
    transform: pga::Motor,
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
