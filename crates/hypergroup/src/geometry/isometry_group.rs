use std::{
    borrow::Cow,
    fmt,
    ops::Index,
    sync::{Arc, LazyLock},
};

use hypermath::{
    APPROX, ApproxHashMap, MotorNearestNeighborMap, Point, Vector,
    approx_collections::hash_map::Entry, pga::Motor,
};
use hypuz_util::ti::{TiVec, TypedIndex, TypedIndexIter};
use itertools::Itertools;

use crate::{
    AbstractGroupLut, GeneratorId, Group, GroupAction, GroupElementId, GroupError, GroupResult,
    PerFactorGroup, PerGenerator, PerGroupElement,
};

/// Isometry group.
///
/// This type is reference-counted and thus cheap to clone.
#[derive(Default, Clone)]
pub struct IsometryGroup {
    group: Group, // uses `Arc` internally
    ndim: u8,
    isometries: Arc<PerFactorGroup<Arc<FactorGroupIsometries>>>,
}

impl fmt::Debug for IsometryGroup {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("IsometryGroup")
            .field("group", &self.group)
            .field("ndim", &self.ndim)
            .finish()
    }
}

impl Index<GeneratorId> for IsometryGroup {
    type Output = Motor;

    fn index(&self, index: GeneratorId) -> &Self::Output {
        self.generator_motor(index)
    }
}

impl IsometryGroup {
    /// Returns the trivial isometry group.
    pub fn trivial() -> Self {
        static TRIVIAL_GROUP: LazyLock<IsometryGroup> = LazyLock::new(|| {
            IsometryGroup::from_factors([]).expect("error constructing trivial isometry group")
        });

        TRIVIAL_GROUP.clone()
    }

    /// Constructs a group from factors.
    pub(crate) fn from_factors(
        factor_groups: impl IntoIterator<Item = (Arc<AbstractGroupLut>, Arc<FactorGroupIsometries>)>,
    ) -> GroupResult<Self> {
        let mut groups = vec![];
        let mut isometries = PerFactorGroup::new();
        for (g, i) in factor_groups {
            groups.push(g);
            isometries.push(i)?;
        }
        Ok(Self {
            group: Group::from_factors(groups)?,
            ndim: isometries.iter_values().map(|g| g.ndim).sum(),
            isometries: Arc::new(isometries),
        })
    }

    /// Constructs a group from a [direct product].
    ///
    /// [direct product]: https://en.wikipedia.org/wiki/Direct_product_of_groups
    pub fn product<'a>(factors: impl IntoIterator<Item = &'a Self>) -> GroupResult<Self> {
        Self::from_factors(factors.into_iter().flat_map(|factor| {
            std::iter::zip(
                factor
                    .group
                    .direct_product_decomposition()
                    .iter_values()
                    .map(Arc::clone),
                factor.isometries.iter_values().map(Arc::clone),
            )
        }))
    }

    /// Returns the smallest number of dimensions possible for the space
    /// containing the isometry group.
    pub fn ndim(&self) -> u8 {
        self.ndim
    }

    /// Returns the [abstract group], without geometry information.
    ///
    /// [abstract group]: https://en.wikipedia.org/wiki/Group_theory#Abstract_groups
    pub fn abstract_group(&self) -> &Group {
        &self.group
    }

    /// Constructs a group from its generators.
    pub fn from_generators(
        label: impl Into<Cow<'static, str>>,
        generators: PerGenerator<Motor>,
    ) -> GroupResult<Self> {
        let ndim = generators
            .iter_values()
            .map(|g| g.ndim())
            .max()
            .unwrap_or(0);

        let mut element_motors = PerGroupElement::from_iter([Motor::ident(ndim)]);
        let mut motor_to_element =
            ApproxHashMap::from_iter(APPROX, [(Motor::ident(ndim), GroupElementId::IDENTITY)]);
        let abstract_group = Group::from_compose_fn(label, generators.len(), |e, g| {
            let new_motor = (&element_motors[e] * &generators[g])
                .canonicalize()
                .ok_or(GroupError::BadMotor)?;
            Ok(match motor_to_element.entry(new_motor) {
                Entry::Occupied(entry) => *entry.get(),
                Entry::Vacant(entry) => {
                    let new_elem_id = element_motors.push(entry.key().clone())?;
                    *entry.insert(new_elem_id)
                }
            })
        })?;

        let nearest_neighbors =
            MotorNearestNeighborMap::new(&element_motors, element_motors.iter_keys());

        Ok(Self {
            group: abstract_group,
            ndim,
            isometries: Arc::new(PerFactorGroup::from_iter([Arc::new(
                FactorGroupIsometries {
                    ndim,
                    element_motors,
                    nearest_neighbors,
                },
            )])),
        })
    }

    /// Flattens a direct product of factor groups into a single factor group
    /// that represents the same product. [`GeneratorId`]s and
    /// [`GroupElementId`]s are compatible between the original group and the
    /// flattened group.
    ///
    /// Note: This enumerates all elements of the group, which may be very slow.
    pub fn flatten(&self) -> GroupResult<Self> {
        if self.isometries.len() <= 1 {
            Ok(self.clone())
        } else {
            Self::from_generators(
                self.group.label(),
                self.generators()
                    .map_ref(|g, _| self.generator_motor(g).clone()),
            )
        }
    }

    /// Flattens the group using [`IsometryGroup::flatten()`] and constructs a
    /// group action from a set of initial points that are then orbited to
    /// produce the complete set.
    ///
    /// When possible, prefer constructing the direct product of group actions
    /// instead of the group action of a direct product to avoid flattening
    /// large groups.
    ///
    /// TODO: revamp this method
    pub fn action_on_initial_points<P: TypedIndex>(
        &self,
        points: &[Point],
    ) -> GroupResult<GroupAction<P>> {
        // TODO: be smart. return a direct product group action

        let generators = self.generators().map_ref(|g, _| self.generator_motor(g));

        let mut ref_point_to_point = TiVec::<P, Point>::new();
        let mut point_to_ref_point = ApproxHashMap::new(APPROX);
        for initial_point in points {
            let init = ref_point_to_point.push(initial_point.clone())?;
            point_to_ref_point.insert(initial_point.clone(), init);
            crate::orbit(init, &generators, |&p, &g| {
                let new_point = g.transform(&ref_point_to_point[p]);
                match point_to_ref_point.entry(new_point) {
                    Entry::Occupied(_) => None,
                    Entry::Vacant(entry) => {
                        let new_id = ref_point_to_point.push(entry.key().clone()).ok()?; // TODO: handle overflow
                        entry.insert(new_id);
                        Some(new_id)
                    }
                }
            });
        }

        // TODO: optimize this. remember results. don't multiply points extra.

        self.group.action(ref_point_to_point.len(), |g, p| {
            *point_to_ref_point
                .get(generators[g].transform(&ref_point_to_point[p]))
                .expect("missing point")
        })
    }

    /// Flattens the group using [`IsometryGroup::flatten()`] and constructs a
    /// group action from a set of points.
    ///
    /// When possible, prefer constructing the direct product of group actions
    /// instead of the group action of a direct product to avoid flattening
    /// large groups.
    ///
    /// TODO: revamp this method
    pub fn action_on_points<P: TypedIndex>(
        &self,
        ref_point_to_point: &TiVec<P, Point>,
    ) -> GroupResult<GroupAction<P>> {
        let generators = self.generators().map_ref(|g, _| self.generator_motor(g));

        let point_to_ref_point = ApproxHashMap::from_iter(
            APPROX,
            ref_point_to_point.iter().map(|(i, p)| (p.clone(), i)),
        );

        self.group.action(ref_point_to_point.len(), |g, p| {
            *point_to_ref_point
                .get(generators[g].transform(&ref_point_to_point[p]))
                .expect("missing point")
        })
    }

    /// Returns the number of elements in the group.
    pub fn element_count(&self) -> usize {
        self.group.element_count()
    }

    /// Returns an iterator over the elements in the group.
    pub fn elements(&self) -> TypedIndexIter<GroupElementId> {
        self.group.elements()
    }

    /// Returns the list of generators used to generate the group.
    pub fn generators(&self) -> &PerGenerator<GroupElementId> {
        &self.group.generators()
    }

    /// Returns the shortest factorization of `element` into generators. Ties
    /// are broken by lexicographical ordering.
    pub fn factorization(
        &self,
        element: GroupElementId,
    ) -> impl Clone + DoubleEndedIterator<Item = GeneratorId> {
        self.group.factorization(element)
    }

    /// Returns the inverse of `element`.
    pub fn inverse(&self, element: GroupElementId) -> GroupElementId {
        self.group.inverse(element)
    }

    /// Returns the composition `a * b`.
    pub fn compose(&self, a: GroupElementId, b: GroupElementId) -> GroupElementId {
        self.group.compose(a, b)
    }

    /// Returns the `i`th power of an element `e`.
    pub fn powi(&self, e: GroupElementId, i: i32) -> GroupElementId {
        self.group.powi(e, i)
    }

    /// Returns the motor for each generator.
    pub fn generator_motors(&self) -> PerGenerator<&Motor> {
        self.generators().map_ref(|g, _| self.generator_motor(g))
    }

    /// Returns the motor for a generator.
    ///
    /// This is an optimized equivalent to `group.motor(group.generators()[g])`.
    pub fn generator_motor(&self, g: GeneratorId) -> &Motor {
        let (factor, g_in_factor) = self.group.generator_within_factor(g);
        let factor_group = &self.group.direct_product_decomposition()[factor];
        &self.isometries[factor].element_motors[factor_group.generators()[g_in_factor]]
    }

    /// Returns the motor for an element.
    pub fn motor(&self, e: GroupElementId) -> Motor {
        // TODO: consider returning Cow<'_, Motor>
        let mut prior_ndim = 0;
        std::iter::zip(
            self.isometries.iter_values(),
            self.group.element_to_factors(e),
        )
        .map(|(factor_group_isometries, element_factor)| {
            let m = lift_by_ndim(factor_group_isometries.ndim, prior_ndim)
                .transform(&factor_group_isometries.element_motors[element_factor]);
            prior_ndim += factor_group_isometries.ndim;
            m
        })
        .reduce(|m1, m2| m1 * m2)
        .unwrap_or_else(|| Motor::ident(self.ndim as u8))
    }

    /// Returns whether an element is a reflection.
    pub fn is_reflection(&self, e: GroupElementId) -> bool {
        let mut is_reflection = false;
        for g in self.factorization(e) {
            is_reflection ^= self.generator_motor(g).is_reflection();
        }
        is_reflection
    }

    /// Returns the element with the nearest motor.
    pub fn nearest(&self, m: &Motor) -> GroupElementId {
        let mut prior_ndim = 0;
        let nearest_factor_elements =
            self.isometries
                .iter_values()
                .map(|factor_group_isometries| {
                    let unlifted_m = lift_by_ndim(factor_group_isometries.ndim, prior_ndim)
                        .reverse()
                        .transform(m);
                    prior_ndim += factor_group_isometries.ndim;
                    *factor_group_isometries
                        .nearest_neighbors
                        .nearest(&unlifted_m)
                        .unwrap_or(&GroupElementId::IDENTITY) // fallback, shouldn't ever happen
                });
        self.group.element_from_factors(nearest_factor_elements)
    }

    /// Returns an element from its motor.
    pub fn element_from_motor(&self, m: &Motor) -> Option<GroupElementId> {
        Some(self.nearest(m)).filter(|&id| self.motor(id).is_equivalent_to(m))
    }
}

pub(crate) struct FactorGroupIsometries {
    ndim: u8,
    element_motors: PerGroupElement<Motor>,
    nearest_neighbors: MotorNearestNeighborMap<GroupElementId>,
}

impl FactorGroupIsometries {
    pub(crate) fn from_generators_unchecked(
        group: &AbstractGroupLut,
        generators: &PerGenerator<Motor>,
    ) -> Self {
        let ndim = generators
            .iter_values()
            .map(|m| m.ndim())
            .max()
            .unwrap_or(1);

        assert_eq!(group.generators().len(), generators.len());

        let mut element_motors =
            PerGroupElement::<Option<Motor>>::new_with_len(group.element_count());
        element_motors[GroupElementId::IDENTITY] = Some(Motor::ident(ndim));
        crate::orbit(
            GroupElementId::IDENTITY,
            &group.generators().iter_keys().collect_vec(),
            |&e, &g| {
                let new_elem = group.compose_elem_generator(e, g);
                let elem_motor = element_motors[e]
                    .as_ref()
                    .expect("no motor for existing element");
                if element_motors[new_elem].is_none() {
                    element_motors[new_elem] = Some(
                        (elem_motor * &generators[g])
                            .canonicalize()
                            .expect("bad motor"),
                    );
                    Some(new_elem)
                } else {
                    debug_assert!(
                        APPROX.eq(
                            element_motors[new_elem].as_ref().expect("missing motor"),
                            &(elem_motor * &generators[g])
                                .canonicalize()
                                .expect("bad motor"),
                        ),
                        "motors do not generate the same group",
                    );
                    None
                }
            },
        );

        let element_motors =
            element_motors.map(|_, m| m.expect("motors do not generate the same group"));

        let nearest_neighbors = MotorNearestNeighborMap::new(&element_motors, group.elements());

        Self {
            ndim,
            element_motors,
            nearest_neighbors,
        }
    }
}

fn lift_by_ndim(ndim: u8, lift_by: u8) -> Motor {
    (0..ndim)
        .map(|i| Motor::rotation_infallible(Vector::unit(i), Vector::unit(i + lift_by)))
        .reduce(|m1, m2| m1 * m2)
        .unwrap_or_else(|| Motor::ident(0))
}

#[cfg(test)]
mod tests {
    use hypermath::{assert_approx_eq, vector};

    use super::*;

    #[test]
    fn test_lift_by_ndim() {
        let init = Motor::rotation_infallible(vector![1.0], vector![1.0, 2.0, 3.0]);

        let expected = Motor::rotation_infallible(vector![1.0], vector![1.0, 2.0, 3.0]);
        assert_approx_eq!(expected, lift_by_ndim(3, 0).transform(&init));

        let expected = Motor::rotation_infallible(vector![0.0, 1.0], vector![0.0, 1.0, 2.0, 3.0]);
        assert_approx_eq!(expected, lift_by_ndim(3, 1).transform(&init));

        let expected =
            Motor::rotation_infallible(vector![0.0, 0.0, 1.0], vector![0.0, 0.0, 1.0, 2.0, 3.0]);
        assert_approx_eq!(expected, lift_by_ndim(3, 2).transform(&init));

        let expected = Motor::rotation_infallible(
            vector![0.0, 0.0, 0.0, 1.0],
            vector![0.0, 0.0, 0.0, 1.0, 2.0, 3.0],
        );
        assert_approx_eq!(expected, lift_by_ndim(3, 3).transform(&init));

        let expected = Motor::rotation_infallible(
            vector![0.0, 0.0, 0.0, 0.0, 1.0],
            vector![0.0, 0.0, 0.0, 0.0, 1.0, 2.0, 3.0],
        );
        assert_approx_eq!(expected, lift_by_ndim(3, 4).transform(&init));
    }
}
