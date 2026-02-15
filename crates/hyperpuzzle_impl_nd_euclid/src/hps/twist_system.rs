use std::fmt;
use std::sync::Arc;

use hypermath::pga::Motor;
use hypermath::prelude::*;
use hyperpuzzle_core::prelude::*;
use hyperpuzzlescript::*;
use itertools::Itertools;

use super::{ArcMut, HpsAxis, HpsOrbitNames, HpsOrbitNamesComponent, HpsSymmetry, HpsTwist, Names};
use crate::PerReferenceVector;
use crate::builder::*;

/// HPS twist system builder.
pub(super) type HpsTwistSystem = ArcMut<TwistSystemBuilder>;
impl_simple_custom_type!(
    HpsTwistSystem = "euclid.TwistSystem",
    field_get = Self::impl_field_get,
    index_get = Self::impl_index_get,
);
impl fmt::Debug for HpsTwistSystem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self}")
    }
}
impl fmt::Display for HpsTwistSystem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}(id = {:?})", self.type_name(), self.lock().id)
    }
}

/// Adds the built-ins.
pub fn define_in(builtins: &mut Builtins<'_>) -> Result<()> {
    builtins.set_custom_ty::<HpsTwistSystem>()?;

    builtins.set_fns(hps_fns![
        #[kwargs(kwargs)]
        fn add_twist(ctx: EvalCtx, axis: HpsAxis, transform: Motor) -> Option<HpsTwist> {
            HpsTwistSystem::get(ctx)?
                .add_symmetric_with_multipliers(ctx, axis, transform, kwargs)?
        }
        #[kwargs(kwargs)]
        fn add_twist(
            ctx: EvalCtx,
            axis: HpsAxis,
            transform: Motor,
            (name, name_span): Names,
        ) -> Option<HpsTwist> {
            if let Some(old_value) = kwargs.insert("name".into(), name.0.at(name_span)) {
                return Err("duplicate `name` argument".at(old_value.span));
            };
            HpsTwistSystem::get(ctx)?
                .add_symmetric_with_multipliers(ctx, axis, transform, kwargs)?
        }

        fn add_twist_direction(
            ctx: EvalCtx,
            name: String,
            (gen_twist, gen_twist_span): Arc<FnValue>,
        ) -> () {
            let this = HpsTwistSystem::get(ctx)?;
            if !ctx.scope.special.sym.is_null() {
                return Err("twist directions cannot use symmetry".at(ctx.caller_span));
            }
            let twists = this.lock();
            if twists.directions.contains_key(&name) {
                ctx.warn(format!("duplicate twist direction name {name:?}"));
            } else {
                let axis_count = twists.axes.len();
                drop(twists);
                let mut twist_seqs = PerAxis::new();
                for id in Axis::iter(axis_count) {
                    let axis = HpsAxis {
                        id,
                        axes: this.axes(),
                    };
                    let fn_ret = gen_twist.call(
                        gen_twist_span,
                        ctx,
                        vec![axis.at(BUILTIN_SPAN)],
                        Map::new(),
                    )?;

                    let twist_seq = unpack_list_one_or_null::<HpsTwist>(fn_ret)?;

                    twist_seqs
                        .push(if twist_seq.is_empty() {
                            None
                        } else {
                            Some(twist_seq.into_iter().map(|twist| twist.id).collect())
                        })
                        .at(ctx.caller_span)?;
                }
                this.lock().directions.insert(name, twist_seqs);
            }
        }

        #[kwargs(
            id: String,
            (symmetry, symmetry_span): HpsSymmetry,
            refs: List,
            init: Vec<String>,
        )]
        fn add_vantage_group(ctx: EvalCtx) -> () {
            let this = HpsTwistSystem::get(ctx)?;
            match this.lock().vantage_groups.entry(id.clone()) {
                indexmap::map::Entry::Occupied(_) => {
                    return Err(
                        format!("vantage group already exists with ID {id:?}").at(ctx.caller_span)
                    );
                }
                indexmap::map::Entry::Vacant(e) => {
                    let mut reference_vectors = PerReferenceVector::new();
                    let mut reference_vector_names = NameSpecBiMapBuilder::new();
                    for pair in refs {
                        let [init, names]: [Value; 2] = pair.to()?;
                        // TODO: support types other than just vectors
                        // (anything that can be orbited? maybe just blades?)
                        let init_vector: Vector = init.to()?;
                        let names: Names = names.to()?;
                        let (transforms, vectors): (Vec<_>, Vec<_>) = symmetry
                            .orbit(init_vector)
                            .into_iter()
                            .map(|(_gen_seq, motor, vector)| (motor, vector))
                            .unzip();
                        let names = names.0.to_strings(ctx, &transforms, ctx.caller_span)?;
                        for (vector, name) in std::iter::zip(vectors, names) {
                            let id = reference_vectors.push(vector).at(ctx.caller_span)?;
                            reference_vector_names.set(id, name).at(ctx.caller_span)?;
                        }
                    }

                    let preferred_reference_vectors = init
                        .into_iter()
                        .map(|s| {
                            reference_vector_names
                                .id_from_string(&s)
                                .ok_or(format!("no reference vector named {s:?}"))
                        })
                        .try_collect()
                        .at(ctx.caller_span)?;

                    e.insert(VantageGroupBuilder {
                        symmetry: symmetry.isometry_group().at(symmetry_span)?,
                        reference_vectors,
                        reference_vector_names,
                        preferred_reference_vectors,
                    });
                }
            }
        }

        #[kwargs(
            name: String,
            group: String,
            view_offset: Option<Motor>,
            transforms: Arc<Map> = Arc::new(Map::new()),
            (axes, axes_span): Value,
            directions: Arc<Map> = Arc::new(Map::new()),
            inherit_directions: Option<Spanned<Arc<FnValue>>>,
        )]
        fn add_vantage_set(ctx: EvalCtx) -> () {
            let this = HpsTwistSystem::get(ctx)?;
            let ndim = this.lock().axes.ndim;
            let ident = Motor::ident(ndim);
            let view_offset = view_offset.unwrap_or_else(|| ident.clone());

            let transforms = Arc::unwrap_or_clone(transforms)
                .into_iter()
                .map(|(k, v)| Ok((k.to_string(), v.to()?)))
                .collect::<Result<Vec<_>>>()?;

            let mut get_inherit_directions = |axis_vector: Vector| -> Result<Option<Motor>> {
                let &Some((ref f, f_span)) = &inherit_directions else {
                    return Ok(None);
                };
                let axis_vector_value = ValueData::Vec(axis_vector).at(ctx.caller_span);
                f.call(f_span, ctx, vec![axis_vector_value], Map::new())?
                    .to::<Option<Motor>>()
            };

            // TODO: refactor and add more ways to specify relative axes & twists

            let mut axes = if axes.is_null() {
                vec![]
            } else if let Ok(s) = axes.as_ref::<str>() {
                if s != "*" {
                    return Err(
                        "invalid string for key `axes`; only \"*\" is allowed".at(axes_span)
                    );
                }
                let axis_count = this.lock().axes.len();
                Axis::iter(axis_count)
                    .map(|axis| -> Result<Option<(String, RelativeAxisBuilder)>> {
                        let this = this.lock();
                        let Some(axis_name) = this.axes.names.get(axis) else {
                            return Ok(None);
                        };

                        let Ok(axis_info) = this.axes.get(axis) else {
                            return Ok(None);
                        };
                        let axis_name_spec = axis_name.spec.clone();
                        let axis_vector = axis_info.vector().clone();
                        drop(this); // Drop before running `get_inherit_directins()`

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
            } else if axes.is::<Map>() {
                axes.unwrap_or_clone_arc::<Map>()?
                    .into_iter()
                    .map(|(k, pair)| {
                        let (transform, (absolute_axis, absolute_axis_span)) =
                            unpack_value_with_optional_transform::<HpsAxis>(pair, ndim)?;
                        let axis_vector = transform.transform(
                            this.lock()
                                .axes
                                .get(absolute_axis.id)
                                .at(absolute_axis_span)?
                                .vector(),
                        );

                        Result::Ok((
                            k.to_string(),
                            RelativeAxisBuilder {
                                absolute_axis: absolute_axis.id,
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

            for (k, v) in Arc::unwrap_or_clone(directions) {
                match axes
                    .iter_mut()
                    .find(|(name_spec, _)| hyperpuzzle_core::name_spec_matches_name(name_spec, &k))
                {
                    Some((_, relative_axis_builder)) => {
                        for (direction_name, pair) in v.unwrap_or_clone_arc::<Map>()? {
                            let (transform, (absolute_twist, _absolute_twist_span)) =
                                unpack_value_with_optional_transform::<HpsTwist>(pair, ndim)?;
                            relative_axis_builder.direction_map.directions.push((
                                direction_name.to_string(),
                                RelativeTwistBuilder {
                                    absolute_twist: absolute_twist.id,
                                    transform,
                                },
                            ));
                        }
                    }
                    None => ctx.warn(format!("no axis named {k:?}")),
                }
            }

            this.lock().vantage_sets.push(VantageSetBuilder {
                name,
                group,
                view_offset,
                transforms,
                axes,
            });
        }
    ])
}

fn unpack_value_with_optional_transform<T: FromValue + CustomValue>(
    pair: Value,
    ndim: u8,
) -> Result<(Motor, Spanned<T>)> {
    if pair.is::<List>() {
        let [t, a]: [Value; 2] = pair.to()?;
        Ok((t.to()?, a.to()?))
    } else if pair.is::<T>() {
        Ok((Motor::ident(ndim), pair.to()?))
    } else {
        Err(pair.type_error(Type::List(None) | T::hps_ty()))
    }
}

impl HpsTwistSystem {
    pub fn get<'a>(ctx: &EvalCtx<'a>) -> Result<&'a Self> {
        ctx.scope.special.twists.as_ref()
    }

    fn impl_field_get(
        &self,
        _span: Span,
        (field, _field_span): Spanned<&str>,
    ) -> Result<Option<ValueData>> {
        Ok(match field {
            "axes" => Some(self.axes().into()),
            _ => {
                if let Some(exported_field) =
                    self.lock().hps_exports.get(field).cloned().map(|v| v.data)
                {
                    Some(exported_field)
                } else {
                    self.twist_from_name(field)?.map(|v| v.into())
                }
            }
        })
    }
    fn impl_index_get(
        &self,
        _ctx: &mut EvalCtx<'_>,
        _span: Span,
        index: Value,
    ) -> Result<ValueData> {
        // TODO: allow indexing by numeric ID
        Ok(self.twist_from_name(index.as_ref::<str>()?)?.into())
    }
    fn twist_from_name(&self, name: &str) -> Result<Option<HpsTwist>> {
        let twists = self.lock();
        match twists.names.id_from_string(name) {
            Some(id) => {
                let twists = self.clone();
                Ok(Some(HpsTwist { id, twists }))
            }
            None => Ok(None),
        }
    }

    /// Adds a new set of twists with symmetry and multipliers.
    pub fn add_symmetric_with_multipliers(
        &self,
        ctx: &mut EvalCtx<'_>,
        axis: HpsAxis,
        transform: Motor,
        kwargs: Map,
    ) -> Result<Option<HpsTwist>> {
        let span = ctx.caller_span;
        let ndim = self.lock().axes.ndim;

        unpack_kwargs!(
            kwargs,
            multipliers: Option<bool>,
            inverse: Option<bool>,
            do_naming: bool = true,
            prefix: Option<Names>,
            name: Option<Names>,
            suffix: Option<Names>,
            inv_name: Option<Names>,
            inv_suffix: Option<Names>,
            name_fn: Option<Arc<FnValue>>,
            qtm: Option<usize>,
            gizmo_pole_distance: Option<Num>,
        );

        if !do_naming {
            for (value_is_some, kwarg_name) in [
                (prefix.is_some(), "prefix"),
                (name.is_some(), "name"),
                (suffix.is_some(), "suffix"),
                (inv_name.is_some(), "inv_name"),
                (inv_suffix.is_some(), "inv_suffix"),
                (name_fn.is_some(), "name_fn"),
            ] {
                if value_is_some {
                    ctx.warn(format!(
                        "`{kwarg_name}` and `do_naming=false` are mutually exclusive"
                    ));
                }
            }
        }

        let prefix = prefix.map(|Names(n)| n);
        let name = name.map(|Names(n)| n);
        let suffix = suffix.map(|Names(n)| n);
        let inv_name = inv_name.map(|Names(n)| n);
        let inv_suffix = inv_suffix.map(|Names(n)| n);

        let gizmo_pole_distance = gizmo_pole_distance.map(|x| x as f32);

        let axis_id = axis.id;
        let prefix = prefix.or_else(|| Some((HpsOrbitNamesComponent::Axis(axis), span).into()));
        let axis = axis_id;

        let inverse = inverse.unwrap_or(ndim == 3);
        let multipliers = multipliers.unwrap_or(ndim == 3);

        let suffix = suffix.unwrap_or_default();
        let inv_suffix = inv_suffix.unwrap_or_else(|| match &inv_name {
            Some(_) => suffix.clone(),
            None => HpsOrbitNames::from("'"),
        });

        if name_fn.is_some() && (name.is_some() || inv_name.is_some()) {
            return Err(
                "when `name_fn` is specified, `name` and `inv_name` must not be specified".at(span),
            );
        }

        let prefix = prefix.unwrap_or_default();
        let name = name.unwrap_or_default();
        let inv_name = inv_name.unwrap_or_else(|| name.clone());

        let qtm = qtm.unwrap_or(1);
        if qtm < 1 {
            ctx.warn("twist has QTM value less than 1");
        }

        if gizmo_pole_distance.is_some() && ndim != 3 && ndim != 4 {
            return Err("twist gizmo is only supported in 3D and 4D".at(span));
        }

        let base_transform = transform;

        let get_name = |ctx: &mut EvalCtx<'_>, i: i32| {
            if let Some(name_fn) = &name_fn {
                let args = vec![ValueData::Num(i as Num).at(span)];
                name_fn
                    .call(span, ctx, args, Map::new())?
                    .to()
                    .map(|Names(n)| n)
            } else if do_naming {
                match i {
                    1 => Ok(prefix.clone() + name.clone() + suffix.clone()),
                    -1 => Ok(prefix.clone() + inv_name.clone() + inv_suffix.clone()),
                    2.. => {
                        let mult = HpsOrbitNames::from(i.to_string().as_str());
                        Ok(prefix.clone() + name.clone() + mult.clone() + suffix.clone())
                    }
                    ..=-2 => {
                        let mult = HpsOrbitNames::from((-i).to_string().as_str());
                        Ok(prefix.clone() + inv_name.clone() + mult.clone() + inv_suffix.clone())
                    }
                    0 => Err("bad twist multiplier".at(span)),
                }
            } else {
                Ok(HpsOrbitNames::default())
            }
        };

        let transform = base_transform.clone();
        let builder = TwistBuilder {
            axis,
            transform,
            qtm,
            gizmo_pole_distance,
            include_in_scrambles: true,
        };
        let twist_name = get_name(ctx, 1)?;
        let first_twist_id = self.add_symmetric(ctx, builder, twist_name)?;
        if inverse {
            let transform = base_transform.reverse();
            let is_equivalent_to_reverse = base_transform.is_self_reverse();
            let twist_name = get_name(ctx, -1)?;
            let builder = TwistBuilder {
                axis,
                transform,
                qtm,
                gizmo_pole_distance: gizmo_pole_distance.filter(|_| ndim > 3),
                include_in_scrambles: !is_equivalent_to_reverse,
            };
            self.add_symmetric(ctx, builder, twist_name)?;
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
                .at(span));
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

            let builder = TwistBuilder {
                axis,
                transform,
                qtm: qtm * i as usize,
                gizmo_pole_distance: None, // no gizmo for multiples
                include_in_scrambles: true,
            };
            let names = get_name(ctx, i)?;
            self.add_symmetric(ctx, builder, names)?;

            if inverse {
                let transform = previous_transform.reverse();
                let is_equivalent_to_reverse = previous_transform.is_self_reverse();
                let builder = TwistBuilder {
                    axis,
                    transform,
                    qtm: qtm * i as usize,
                    gizmo_pole_distance: None, // no gizmo for multiples
                    include_in_scrambles: !is_equivalent_to_reverse,
                };
                let names = get_name(ctx, -i)?;
                self.add_symmetric(ctx, builder, names)?;
            }
        }

        Ok(first_twist_id)
    }

    // Adds a set of symmetric twists.
    pub fn add_symmetric(
        &self,
        ctx: &mut EvalCtx<'_>,
        mut builder: TwistBuilder,
        names: HpsOrbitNames,
    ) -> Result<Option<HpsTwist>> {
        let span = ctx.caller_span;
        let ctx_symmetry = HpsSymmetry::get(ctx)?;

        let mut first_twist = None;

        match ctx_symmetry {
            Some(sym) => {
                let this = self.lock();
                let axis_vector = this.axes.get(builder.axis).at(span)?.vector().clone();
                let (transforms, orbit_elements): (Vec<_>, Vec<_>) = sym
                    .orbit(GeometricTwistKey {
                        axis_vector,
                        transform: builder.transform.clone(),
                    })
                    .into_iter()
                    .map(|(_gen_seq, transform, orbit_element)| (transform, orbit_element))
                    .unzip();

                drop(this); // unlock mutex before `to_strings()`
                let names = names.to_strings(ctx, &transforms, span)?;
                let mut this = self.lock();

                for (key, name) in std::iter::zip(orbit_elements, names) {
                    builder.axis =
                        super::axis_from_vector(&this.axes, &key.axis_vector).at(span)?;
                    builder.transform = key.transform;
                    let new_twist = this.add(builder.clone(), name, &mut ctx.warnf()).at(span)?;
                    if first_twist.is_none() {
                        first_twist = Some(new_twist);
                    }
                }
            }
            None => {
                let mut names = names.to_strings(ctx, &[Motor::ident(ctx.ndim()?)], span)?;
                let mut this = self.lock();
                first_twist = Some(
                    this.add(builder, names.next().flatten(), &mut ctx.warnf())
                        .at(span)?,
                );
            }
        };

        Ok(first_twist.flatten().map(|id| HpsTwist {
            id,
            twists: self.clone(),
        }))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct GeometricTwistKey {
    pub axis_vector: Vector,
    pub transform: Motor,
}
impl ApproxEq for GeometricTwistKey {
    fn approx_eq(&self, other: &Self, prec: Precision) -> bool {
        prec.eq(&self.axis_vector, &other.axis_vector) && prec.eq(&self.transform, &other.transform)
    }
}
impl ApproxInternable for GeometricTwistKey {
    fn intern_floats<F: FnMut(&mut f64)>(&mut self, f: &mut F) {
        self.axis_vector.intern_floats(f);
        self.transform.intern_floats(f);
    }
}
impl ApproxHash for GeometricTwistKey {
    fn interned_eq(&self, other: &Self) -> bool {
        self.axis_vector.interned_eq(&other.axis_vector)
            && self.transform.interned_eq(&other.transform)
    }

    fn interned_hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.axis_vector.interned_hash(state);
        self.transform.interned_hash(state);
    }
}
impl Ndim for GeometricTwistKey {
    fn ndim(&self) -> u8 {
        std::cmp::max(self.axis_vector.ndim(), self.transform.ndim())
    }
}
impl TransformByMotor for GeometricTwistKey {
    fn transform_by(&self, m: &Motor) -> Self {
        let t = m.transform(&self.transform);
        Self {
            axis_vector: m.transform(&self.axis_vector),
            transform: if m.is_reflection() { t.reverse() } else { t },
        }
    }
}

fn unpack_list_one_or_null<T>(value: Value) -> Result<Vec<T>>
where
    T: FromValue,
    for<'a> &'a T: FromValueRef<'a>,
{
    if value.is_null() {
        Ok(vec![])
    } else if value.is::<T>() {
        Ok(vec![value.to()?])
    } else if value.is::<List>() {
        value.to()
    } else {
        Err(value.type_error(Type::Null | T::hps_ty() | Type::List(Some(Box::new(T::hps_ty())))))
    }
}
