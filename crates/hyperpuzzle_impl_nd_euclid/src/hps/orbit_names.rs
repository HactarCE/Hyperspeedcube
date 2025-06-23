use std::{fmt, sync::Arc};

use hypermath::{ApproxHashMap, Point, Vector, pga::Motor};
use hyperpuzzle_core::{Axis, NameSpec, NameSpecMap, Twist};
use hyperpuzzlescript::{
    CustomValue, ErrorExt, EvalCtx, Map, Result, Scope, Span, Spanned, Str, TryEq, Type, Value,
    hps_fns, impl_simple_custom_type,
};
use hypershape::{GenSeq, GeneratorId};
use itertools::Itertools;
use parking_lot::Mutex;

use super::{HpsAxis, HpsSymmetry, HpsTwist};
use crate::{
    TwistKey,
    builder::{AxisSystemBuilder, TwistSystemBuilder},
};

/// Adds the built-ins to the scope.
pub fn define_in(scope: &Scope) -> Result<()> {
    scope.register_custom_type::<HpsOrbitNames>();

    scope.register_builtin_functions(hps_fns![
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

    scope.register_builtin_functions(hps_fns![
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

#[derive(Default, Clone)]
pub struct HpsOrbitNames {
    components: Vec<Spanned<HpsOrbitNamesComponent>>,
}
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
impl_simple_custom_type!(HpsOrbitNames = "euclid.OrbitNames");
impl From<Spanned<HpsOrbitNamesComponent>> for HpsOrbitNames {
    fn from(value: Spanned<HpsOrbitNamesComponent>) -> Self {
        Self {
            components: vec![value],
        }
    }
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
            match component {
                HpsOrbitNamesComponent::Str(new_str) => {
                    for s in &mut strings {
                        *s += &**new_str;
                    }
                }
                HpsOrbitNamesComponent::Axis(axis) => {
                    let twists = axis.twists.lock();
                    let axes = &twists.axes;
                    let init_vector = axes.get(axis.id).at(component_span)?.vector();
                    for (s, t) in std::iter::zip(&mut strings, transforms) {
                        let transformed_axis_name =
                            axis_name_from_vector(axes, &t.transform(init_vector)).at(span)?;
                        s.push_str(&transformed_axis_name.spec);
                    }
                }
                HpsOrbitNamesComponent::Twist(twist) => {
                    let twists = twist.twists.lock();
                    let axes = &twists.axes;
                    let init_twist = twists.get(twist.id).at(component_span)?;
                    let init_vector = axes.get(init_twist.axis).at(component_span)?.vector();
                    let init_transform = &init_twist.transform;
                    for (s, t) in std::iter::zip(&mut strings, transforms) {
                        let transformed_axis =
                            axis_from_vector(axes, &t.transform(init_vector)).at(span)?;
                        let transformed_transform = &t.transform(init_transform);
                        let key = TwistKey::new(transformed_axis, transformed_transform)
                            .ok_or(OrbitNamesError::BadTwistTransform)
                            .at(span)?;
                        let transformed_twist_name =
                            twist_name_from_key(&*twists, &key).at(span)?;
                        s.push_str(&transformed_twist_name.spec);
                    }
                }
                HpsOrbitNamesComponent::Cosets(lazy_coset_map) => {
                    // Compute cosets map.
                    let mut lazy_coset_map_guard = lazy_coset_map.lock();
                    let (initial_coset_point, coset_names) = lazy_coset_map_guard.compute(ctx)?;

                    for (s, t) in std::iter::zip(&mut strings, transforms) {
                        let transformed_coset_point = t.transform(initial_coset_point);
                        let coset_str = coset_names
                            .get(&transformed_coset_point)
                            .ok_or(OrbitNamesError::MissingCoset(transformed_coset_point))
                            .at(span)?;
                        s.push_str(coset_str);
                    }
                }
            }
        }
        Ok(strings)
    }
}

#[derive(Debug, Clone)]
pub(super) enum HpsOrbitNamesComponent {
    Str(Str),
    Axis(HpsAxis),
    Twist(HpsTwist),
    Cosets(Arc<Mutex<LazyCosetMap>>),
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

#[derive(thiserror::Error, Debug, Clone)]
enum OrbitNamesError {
    #[error("no axis with vector {0}")]
    NoAxis(Vector),
    #[error("axis {0} with vector {1} has no name")]
    UnnamedAxis(Axis, Vector),
    #[error("no {0}")]
    NoTwist(TwistKey),
    #[error("{0} has no name")]
    UnnamedTwist(Twist, TwistKey),
    #[error("bad twist transform")]
    BadTwistTransform,
    #[error("missing coset {0}")]
    MissingCoset(Point),
}

fn axis_from_vector(axes: &AxisSystemBuilder, vector: &Vector) -> Result<Axis, OrbitNamesError> {
    axes.vector_to_id(&vector)
        .ok_or_else(|| OrbitNamesError::NoAxis(vector.clone()))
}

fn axis_name_from_vector<'a>(
    axes: &'a AxisSystemBuilder,
    vector: &Vector,
) -> Result<&'a NameSpec, OrbitNamesError> {
    let id = axis_from_vector(axes, vector)?;
    axes.names
        .get(id)
        .ok_or_else(|| OrbitNamesError::UnnamedAxis(id, vector.clone()))
}

fn twist_name_from_key<'a>(
    twists: &'a TwistSystemBuilder,
    key: &TwistKey,
) -> Result<&'a NameSpec, OrbitNamesError> {
    let id = twists
        .key_to_id(key)
        .ok_or_else(|| OrbitNamesError::NoTwist(key.clone()))?;
    twists
        .names
        .get(id)
        .ok_or_else(|| OrbitNamesError::UnnamedTwist(id, key.clone()))
}
