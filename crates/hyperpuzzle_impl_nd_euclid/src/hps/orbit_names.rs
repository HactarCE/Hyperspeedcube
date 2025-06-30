use std::fmt;
use std::ops::Add;
use std::sync::Arc;

use hypermath::pga::Motor;
use hypermath::{ApproxHashMap, Point};
use hyperpuzzle_core::NameSpecMap;
use hyperpuzzlescript::{
    Builtins, CustomValue, ErrorExt, EvalCtx, FnValue, FromValue, Map, Result, Span, Spanned, Str,
    TryEq, Type, TypeOf, Value, ValueData, hps_fns, impl_simple_custom_type, impl_ty,
};
use hypershape::{GenSeq, GeneratorId};
use itertools::Itertools;
use parking_lot::Mutex;

use super::{HpsAxis, HpsEuclidError, HpsSymmetry, HpsTwist};

#[derive(Debug, Clone)]
pub struct Names(pub HpsOrbitNames);
impl_ty!(Names = Type::Str | HpsOrbitNames::hps_ty() | Type::Fn);
impl FromValue for Names {
    fn from_value(value: Value) -> Result<Self> {
        let span = value.span;
        if value.is::<str>() {
            Ok(Self(HpsOrbitNames::from((value.to::<Str>()?.into(), span))))
        } else if value.is::<HpsOrbitNames>() {
            Ok(Self(value.to::<HpsOrbitNames>()?))
        } else if value.is::<FnValue>() {
            Ok(Self(HpsOrbitNames::from((
                HpsOrbitNamesComponent::Fn(value.to::<Arc<FnValue>>()?),
                span,
            ))))
        } else {
            Err(value.type_error(Self::hps_ty()))
        }
    }
}

#[derive(Default, Clone)]
pub struct HpsOrbitNames {
    components: Vec<Spanned<HpsOrbitNamesComponent>>,
}
impl_simple_custom_type!(HpsOrbitNames = "euclid.OrbitNames");
impl fmt::Debug for HpsOrbitNames {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct(self.type_name()).finish_non_exhaustive()
    }
}
impl fmt::Display for HpsOrbitNames {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct(self.type_name()).finish_non_exhaustive()
    }
}
impl TryEq for HpsOrbitNames {
    fn try_eq(&self, _other: &Self) -> Option<bool> {
        None // fail
    }
}
impl From<Spanned<HpsOrbitNamesComponent>> for HpsOrbitNames {
    fn from(value: Spanned<HpsOrbitNamesComponent>) -> Self {
        Self {
            components: vec![value],
        }
    }
}
impl From<&str> for HpsOrbitNames {
    fn from(value: &str) -> Self {
        Self::from((
            HpsOrbitNamesComponent::Str(value.into()),
            hyperpuzzlescript::BUILTIN_SPAN,
        ))
    }
}

/// Adds the built-ins.
pub fn define_in(builtins: &mut Builtins<'_>) -> Result<()> {
    builtins.set_custom_ty::<HpsOrbitNames>()?;

    builtins.set_fns(hps_fns![
        ("$", |ctx, axis: HpsAxis| -> HpsOrbitNames {
            HpsOrbitNames::from((axis.into(), ctx.caller_span))
        }),
        ("$", |ctx, twist: HpsTwist| -> HpsOrbitNames {
            HpsOrbitNames::from((twist.into(), ctx.caller_span))
        }),
        (
            "++",
            |_, (a, a_span): Str, b: HpsOrbitNames| -> HpsOrbitNames {
                let mut ret = b;
                ret.components.insert(0, (a.into(), a_span));
                ret
            }
        ),
        (
            "++",
            |_, a: HpsOrbitNames, (b, b_span): Str| -> HpsOrbitNames {
                let mut ret = a;
                ret.components.push((b.into(), b_span));
                ret
            }
        ),
        (
            "++",
            |_, a: HpsOrbitNames, b: HpsOrbitNames| -> HpsOrbitNames {
                let mut ret = a;
                ret.components.extend(b.components);
                ret
            }
        ),
    ])?;

    builtins.set_fns(hps_fns![
        fn names(
            ctx: EvalCtx,
            symmetry: HpsSymmetry,
            initial_coset_point: Point,
            (names_map, names_map_span): Arc<Map>,
        ) -> HpsOrbitNames {
            HpsOrbitNames::from((
                HpsOrbitNamesComponent::Cosets(Arc::new(Mutex::new(LazyCosetMap::Uninit {
                    symmetry,
                    initial_coset_point,
                    names_map,
                    names_map_span,
                }))),
                ctx.caller_span,
            ))
        }
    ])?;

    Ok(())
}

impl HpsOrbitNames {
    pub fn to_strings(
        &self,
        ctx: &mut EvalCtx<'_>,
        transforms: &[Motor],
        span: Span,
    ) -> Result<Vec<String>> {
        let mut strings = vec![String::new(); transforms.len()];
        for &(ref component, component_span) in &self.components {
            let strings_and_transforms = std::iter::zip(&mut strings, transforms);
            match component {
                HpsOrbitNamesComponent::Str(new_str) => {
                    for s in &mut strings {
                        *s += &**new_str;
                    }
                }
                HpsOrbitNamesComponent::Axis(axis) => {
                    let axes = axis.axes.lock();
                    for (s, t) in strings_and_transforms {
                        let transformed_axis =
                            super::transform_axis(span, &axes, t, (axis.id, component_span))?;
                        let transformed_axis_name =
                            super::axis_name(span, &axes, transformed_axis)?;
                        s.push_str(&transformed_axis_name.spec);
                    }
                }
                HpsOrbitNamesComponent::Twist(twist) => {
                    let twists = twist.twists.lock();
                    for (s, t) in strings_and_transforms {
                        let transformed_twist =
                            super::transform_twist(span, &twists, t, (twist.id, component_span))?;
                        let transformed_twist_name =
                            super::twist_name(span, &twists, transformed_twist)?;
                        s.push_str(&transformed_twist_name.spec);
                    }
                }
                HpsOrbitNamesComponent::Cosets(lazy_coset_map) => {
                    // Compute cosets map.
                    let mut lazy_coset_map_guard = lazy_coset_map.lock();
                    let (initial_coset_point, coset_names) = lazy_coset_map_guard.compute(ctx)?;

                    for (s, t) in strings_and_transforms {
                        let transformed_coset_point = t.transform(initial_coset_point);
                        let coset_str = coset_names
                            .get(&transformed_coset_point)
                            .ok_or(HpsEuclidError::MissingCoset(transformed_coset_point))
                            .at(span)?;
                        s.push_str(coset_str);
                    }
                }
                HpsOrbitNamesComponent::Fn(f) => {
                    for (s, t) in strings_and_transforms {
                        let args = vec![ValueData::EuclidTransform(t.clone()).at(span)];
                        s.push_str(
                            f.call(component_span, ctx, args, Map::new())?
                                .as_ref::<str>()?,
                        );
                    }
                }
            }
        }
        Ok(strings)
    }

    pub fn is_empty(&self) -> bool {
        self.components
            .iter()
            .all(|(component, _span)| match component {
                HpsOrbitNamesComponent::Str(s) => s.is_empty(),
                HpsOrbitNamesComponent::Axis(_)
                | HpsOrbitNamesComponent::Twist(_)
                | HpsOrbitNamesComponent::Cosets(_)
                | HpsOrbitNamesComponent::Fn(_) => false,
            })
    }
}

#[derive(Debug, Clone)]
pub(super) enum HpsOrbitNamesComponent {
    Str(Str),
    Axis(HpsAxis),
    Twist(HpsTwist),
    Cosets(Arc<Mutex<LazyCosetMap>>),
    Fn(Arc<FnValue>),
}
impl From<Str> for HpsOrbitNamesComponent {
    fn from(value: Str) -> Self {
        Self::Str(value)
    }
}
impl From<HpsAxis> for HpsOrbitNamesComponent {
    fn from(value: HpsAxis) -> Self {
        Self::Axis(value)
    }
}
impl From<HpsTwist> for HpsOrbitNamesComponent {
    fn from(value: HpsTwist) -> Self {
        Self::Twist(value)
    }
}
impl From<Arc<FnValue>> for HpsOrbitNamesComponent {
    fn from(value: Arc<FnValue>) -> Self {
        Self::Fn(value)
    }
}
impl Add<HpsOrbitNames> for HpsOrbitNames {
    type Output = HpsOrbitNames;

    fn add(mut self, rhs: Self) -> Self::Output {
        self.components.extend(rhs.components);
        self
    }
}

#[derive(Debug)]
pub(super) enum LazyCosetMap {
    Uninit {
        symmetry: HpsSymmetry,
        initial_coset_point: Point,
        names_map: Arc<Map>,
        names_map_span: Span,
    },
    Init {
        initial_coset_point: Point,
        coset_names: ApproxHashMap<Point, String>,
    },
}
impl LazyCosetMap {
    fn compute(
        &mut self,
        ctx: &mut EvalCtx<'_>,
    ) -> Result<(&Point, &ApproxHashMap<Point, String>)> {
        if let LazyCosetMap::Uninit {
            symmetry,
            initial_coset_point,
            names_map,
            names_map_span,
        } = self
        {
            let initial_coset_point = initial_coset_point.clone();

            let mut name_to_index = NameSpecMap::<usize>::new();
            let mut canonical_names = vec![];
            let mut key_value_dependencies = vec![];

            for (index, (k, v)) in names_map.iter().enumerate() {
                let canonical_name = name_to_index
                    .insert(k, &index)
                    .map_err(|e| format!("invalid name spec {k:?}: {e}"))
                    .at(*names_map_span)?;
                canonical_names.push(canonical_name.clone());

                let mut array: Vec<Value> = v.clone().to()?;
                let mut init_name = None;
                match array.pop() {
                    None => (),
                    Some(last_elem) => {
                        if let Ok(s) = last_elem.as_ref::<str>() {
                            init_name = Some(s.to_string());
                        } else if let Ok(_generator) = last_elem.ref_to().map(GeneratorId) {
                            array.push(last_elem);
                        } else {
                            return Err(last_elem.type_error(Type::Nat | Type::Str));
                        }
                    }
                }
                let gen_seq = array
                    .into_iter()
                    .map(|v| v.to().map(GeneratorId))
                    .try_collect()?;
                let relative_transform = symmetry.motor_for_gen_seq(&GenSeq(gen_seq), v.span)?;

                key_value_dependencies.push((canonical_name, (relative_transform, init_name)));
            }

            let canonicalize = |s: &mut String| {
                if let Some(&i) = name_to_index.get(s) {
                    *s = canonical_names[i].clone();
                }
            };

            // Canonicalize names in `key_value_dependencies`.
            for (_key, (_relative_transform, end)) in &mut key_value_dependencies {
                if let Some(ending_name) = end {
                    canonicalize(ending_name);
                }
            }

            // Resolve lazy evaluation.
            let coset_names = hyperpuzzle_core::util::lazy_resolve(
                key_value_dependencies,
                |t1, t2| t1 * t2,
                ctx.warnf(),
            )
            .into_iter()
            .map(|(name, transform)| (transform.transform(&initial_coset_point), name))
            .collect();

            *self = Self::Init {
                initial_coset_point,
                coset_names,
            };
        }

        let Self::Init {
            initial_coset_point,
            coset_names,
        } = self
        else {
            unreachable!("coset map should be initialized");
        };
        Ok((initial_coset_point, coset_names))
    }
}
