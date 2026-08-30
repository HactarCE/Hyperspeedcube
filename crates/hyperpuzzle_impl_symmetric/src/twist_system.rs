use std::f64::consts::TAU;
use std::num::NonZeroI32;
use std::sync::Arc;

use eyre::{OptionExt, Result, bail};
use hypergroup::{
    ConjugateCoset, ConjugateSubgroupConstraintSolver, GroupAction, SubgroupConstraintSolver,
};
use hypermath::pga::Motor;
use hypermath::prelude::*;
use hyperpuzzle_core::Component;
use hyperpuzzle_core::group::{GroupElementId, IsometryGroup};
use hyperpuzzle_core::prelude::*;
use hyperpuzzle_impl_nd_euclid::NdEuclidAxisVectors;
use hypuz_notation::family::JumbleSuffix;
use hypuz_notation::{Str, Transform};
use hypuz_util::FloatMinMaxByIteratorExt;
use itertools::Itertools;
use parking_lot::Mutex;
use rand::{Rng, RngExt};
use smallvec::{SmallVec, smallvec};

use crate::{NamedPoint, NamedPointSet, PerNamedPoint, StabilizerFamily};

hypuz_util::typed_index_struct! {
    /// ID of a jumbling stop within an axis.
    pub struct JumbleStop(u16);
}

/// List containing a value per jumble stop.
pub type PerJumbleStop<T> = TiVec<JumbleStop, T>;

/// Simulation data for a symmetric puzzle.
///
/// This type is relatively cheap to clone.
// TODO: how many of these fields are needed?
#[derive(Debug, Clone)]
pub struct SymmetricTwistSystemComponent {
    /// Axis system.
    pub axes: Arc<AxisSystem>,
    /// Grip group, which is the symmetry group of the axis system.
    pub group: IsometryGroup,
    /// Possibly-incomplete list of normal vectors for mirror planes bounding
    /// the fundamental region of the grip group.
    ///
    /// This is used for optimization purposes only. It is always acceptable for
    /// this list to be empty.
    pub fundamental_region_mirrors: Vec<Vector>,
    /// Action of the grip group on the axes.
    pub axis_action: GroupAction<Axis>,

    /// For each axis: a group element that transforms the first axis in its
    /// orbit to that axis, and an index into [`Self::axis_orbits`]
    /// corresponding to the axis orbit.
    ///
    /// Each entry in this list describes a [conjugate subgroup], where the
    /// subgroup is determined by the first axis in the orbit.
    ///
    /// The conjugating element is deterministic, based only on the names of the
    /// axes and named points.
    ///
    /// [conjugate subgroup]:
    ///     https://mathworld.wolfram.com/ConjugateSubgroup.html
    pub axis_undeorbiters: Arc<PerAxis<(GroupElementId, usize)>>,
    /// Data for each axis orbit.
    pub axis_orbits: Arc<Vec<SymmetricTwistSystemAxisOrbit>>,
    /// Vector for each axis.
    pub axis_vectors: Arc<NdEuclidAxisVectors>,

    /// Action of the grip group on the named points.
    pub named_point_action: GroupAction<NamedPoint>,
    /// Named point names.
    pub named_point_names: Arc<Names<NamedPoint>>,
    /// Physical location of each named point, for constructing simple direct
    /// rotations from one named point to another.
    pub named_point_vectors: Arc<PerNamedPoint<Vector>>,
}

impl Component<TwistSystem> for SymmetricTwistSystemComponent {}

impl SymmetricTwistSystemComponent {
    /// Returns the number of dimensions of the space containing the puzzle.
    pub fn ndim(&self) -> u8 {
        self.group.ndim()
    }

    /// Returns the motor for a twist.
    pub fn twist_motor(&self, twist: &Move) -> Result<Motor> {
        let (_axis, element, jumble_offset) = self.resolve_twist(twist)?;
        let mut ret = self.group.motor(element);
        if let Some((jumble_motor, _jumble_angle)) = jumble_offset {
            ret = jumble_motor * ret;
        }
        Ok(ret)
    }

    /// Resolves a twist to an axis and a group element, with an optional jumble
    /// offset expressed as a motor and an angle.
    pub fn resolve_twist(
        &self,
        twist: &Move,
    ) -> Result<(Axis, GroupElementId, Option<(Motor, Float)>), TwistError> {
        let (axis, element, jumble_angle) = self.resolve_twist_transform(&twist.transform)?;
        Ok((
            axis,
            self.group.powi(element, twist.multiplier.0),
            jumble_angle.map(|angle| {
                let angle =
                    hypermath::util::canonicalize_angle(angle * twist.multiplier.0 as Float);
                let v = &self.axis_vectors.vectors_by_id[axis];
                let jumble_transform = Motor::from_angle_around_3d_vector(v, -angle); // negate for clockwise
                (jumble_transform, angle)
            }),
        ))
    }

    /// Resolves a twist transform to an axis and a group element, with an
    /// optional jumble angle.
    pub fn resolve_twist_transform(
        &self,
        transform: &Transform,
    ) -> Result<(Axis, GroupElementId, Option<Float>), TwistError> {
        let Some(axis) = self.axes.names.lookup(&transform.family) else {
            let separator = '_'; // TODO: correct number of underscores (maybe none)
            if let Some((primary_axis_str, rest)) = transform.family.split_once(separator)
                && let Some(primary) = self.axes.names.lookup(primary_axis_str)
            {
                // 3D jumbling notation
                let (_, axis_orbit) = self.axis_undeorbiters[primary];
                if let Some(jumble_data) = &self.axis_orbits[axis_orbit].jumble_data
                    && let Ok(jumble_suffix) = rest.parse()
                    && let Some(angle) = jumble_data.suffix_to_angle(jumble_suffix)
                {
                    return Ok((primary, GroupElementId::IDENTITY, Some(angle)));
                }

                // 4D stabilizer notation
                if let Some(secondary) = rest
                    .split(separator)
                    .map(|s| self.named_point_names.lookup(s))
                    .collect::<Option<_>>()
                    .and_then(|axes| NamedPointSet::new(axes).ok())
                    && let Some(unit_twist) = self
                        .resolve_stabilizer_twist_transform(StabilizerFamily { primary, secondary })
                {
                    return Ok((primary, unit_twist.element, None));
                }
            }

            return Err(TwistError::UnknownAxis(transform.family.clone()));
        };

        if transform.constraints.is_none()
            && let Some(unit_twist) = self.resolve_stabilizer_twist_transform(StabilizerFamily {
                primary: axis,
                secondary: NamedPointSet::EMPTY,
            })
        {
            return Ok((axis, unit_twist.element, None)); // 3D stabilizer notation
        }

        let constraint_set = self.constraints_from_notation(
            transform
                .constraints
                .as_ref()
                .unwrap_or(&hypuz_notation::ConstraintSet::default()),
        )?;

        let (conjugating_element, orbit_index) = self.axis_undeorbiters[axis];
        let mut subgroup_solver_guard = self.axis_orbits[orbit_index].subgroup_solver.lock();
        let coset =
            ConjugateSubgroupConstraintSolver::new(conjugating_element, &mut subgroup_solver_guard)
                .solve(constraint_set.clone())
                .ok_or(TwistError::UnsatisfiableConstraints)?;

        let rotation_count = self.count_rotations_in_coset(&coset);

        let element = if rotation_count == 0 {
            return Err(TwistError::UnsatisfiableConstraints);
        } else if rotation_count == 1
            && let Ok(unambiguous_rotation_in_coset) = self.rotations_in_coset(&coset).exactly_one()
        {
            unambiguous_rotation_in_coset
        } else if let [single_constraint] = constraint_set.constraints.as_slice() {
            let direct_rotation = Motor::rotation(
                &self.named_point_vectors[single_constraint.from],
                &self.named_point_vectors[single_constraint.to],
            )
            .ok_or(TwistError::Ambiguous180)?;
            let element = self
                .group
                .element_from_motor(&direct_rotation)
                .ok_or(TwistError::DirectRotationDoesNotExist)?;

            if self.axis_action.act(element, axis) == axis {
                element
            } else {
                return Err(TwistError::DirectRotationDoesNotFixAxis);
            }
        } else {
            return Err(TwistError::Underconstrained {
                coset_size: rotation_count,
            });
        };

        if element == GroupElementId::IDENTITY {
            return Err(TwistError::Identity);
        }

        Ok((axis, element, None))
    }

    /// Resolves a stabilizer family to a unique minimal clockwise twist.
    pub fn resolve_stabilizer_twist_transform(
        &self,
        stabilizer_family: StabilizerFamily,
    ) -> Option<UniqueMinimalClockwiseGenerator> {
        let (conjugating_element, orbit_index) = self.axis_undeorbiters[stabilizer_family.primary];
        let axis_orbit = &self.axis_orbits[orbit_index];
        let mut subgroup_solver = axis_orbit.subgroup_solver.lock();

        let transformed_secondary = stabilizer_family.secondary.transform_by_group_element(
            &self.named_point_action,
            self.group.inverse(conjugating_element),
        );

        for (candidate_secondary, unit_twist, _) in &axis_orbit.stabilizer_twists {
            if stabilizer_family.secondary.len() == candidate_secondary.len()
                && let Some(coset) = subgroup_solver.solve(&hypergroup::ConstraintSet::from_iter(
                    std::iter::zip(candidate_secondary, &transformed_secondary)
                        .map(|(from, to)| hypergroup::Constraint { from, to }),
                ))
            {
                // The coset stabilizes the twist transform, so it doesn't
                // matter which element we take from it.
                let coset_representative = self
                    .group
                    .compose(conjugating_element, coset.arbitrary_element());
                let minimal_stabilizer = self
                    .group
                    .conjugate(coset_representative, unit_twist.element);
                return Some(UniqueMinimalClockwiseGenerator {
                    element: if self.group.is_reflection(coset_representative) {
                        self.group.inverse(minimal_stabilizer)
                    } else {
                        minimal_stabilizer
                    },
                    order: unit_twist.order,
                });
            }
        }

        None
    }

    /// Returns a constraint set specifying a random non-identity transformation
    /// of an axis.
    ///
    /// Returns `None` if there is no such constraint set. Returns
    /// `Some(ConstraintSet::EMPTY)` if there is only one such transformation
    /// and so no constraints are needed.
    pub fn random_constraints_on_axis(
        &self,
        rng: &mut dyn Rng,
        axis: Axis,
    ) -> Option<hypuz_notation::ConstraintSet> {
        let (conjugating_element, orbit_index) = self.axis_undeorbiters[axis];
        let mut subgroup_solver_guard = self.axis_orbits[orbit_index].subgroup_solver.lock();
        let mut solver =
            ConjugateSubgroupConstraintSolver::new(conjugating_element, &mut subgroup_solver_guard);

        let coset = solver.solve(hypergroup::ConstraintSet::EMPTY)?;
        let random_rotation = match self.count_rotations_in_coset(&coset) {
            0 => return None, // impossible! must contain identity
            1 => return None, // only contains identity
            2 => {
                // only one non-identity element; just return it
                self.rotations_in_coset(&coset)
                    .find(|&e| e != GroupElementId::IDENTITY)?
            }
            _ => {
                // Loop until we find a non-identity element. There must be at least 2
                // of them, so at worst we have a 2/3 chance of finding one.
                let mut random_elements = std::iter::from_fn(|| {
                    solver.select(hypergroup::ConstraintSet::EMPTY, |n| rng.random_range(0..n))
                });
                let mut random_rotations = std::iter::from_fn(|| {
                    let (_, candidate_1) = random_elements.next()?;
                    if self.group.is_reflection(candidate_1) {
                        let (_, candidate_2) = random_elements.next()?;
                        if self.group.is_reflection(candidate_2) {
                            Some(self.group.compose(candidate_1, candidate_2)) // refl * refl = rot
                        } else {
                            Some(candidate_2) // rot
                        }
                    } else {
                        Some(candidate_1) // rot
                    }
                });
                random_rotations.find(|&e| e != GroupElementId::IDENTITY)?
            }
        };

        let mut constraints =
            solver.constraints_for_element(hypergroup::ConstraintSet::EMPTY, random_rotation)?;

        // Try removing the last constraint, since it is often unnecessary for
        // chiral puzzles. This isn't perfect but it covers the vast majority of
        // cases.
        let mut constraints_minus_one = constraints.clone();
        constraints_minus_one.constraints.pop();
        if solver
            .solve(constraints_minus_one)
            .is_some_and(|coset| self.count_rotations_in_coset(&coset) == 1)
        {
            constraints.constraints.pop();
        }

        Some(self.constraints_to_notation(constraints))
    }

    /// Returns the number of rotations in a coset without enumerating the
    /// entire coset.
    fn count_rotations_in_coset(&self, coset: &ConjugateCoset) -> usize {
        // Does the coset have any reflections and/or rotations?
        if coset
            .subgroup
            .generators
            .iter()
            .any(|g| self.group.is_reflection(*g))
        {
            coset.subgroup.element_count / 2 // reflections and rotations
        } else if self.group.is_reflection(coset.lhs) == self.group.is_reflection(coset.rhs) {
            coset.subgroup.element_count // rotations only
        } else {
            0 // reflections only
        }
    }

    /// Returns the rotation elements within a coset.
    ///
    /// This is **not** performant for large cosets.
    fn rotations_in_coset(&self, coset: &ConjugateCoset) -> impl Iterator<Item = GroupElementId> {
        coset
            .elements()
            .into_iter()
            .filter(|&e| !self.group.is_reflection(e))
    }

    /// Returns whether an axis has any non-identity twist transforms available.
    ///
    /// On an actual puzzle, there may still be no twists available because the
    /// axis has no layers.
    pub fn axis_has_twists(&self, axis: Axis) -> bool {
        self.axis_stabilizer(axis)
            .is_some_and(|coset| self.count_rotations_in_coset(&coset) > 1)
    }

    /// Returns the coset of twist transforms on an axis, or `None` if there are
    /// none.
    ///
    /// These should be filtered to include only rotations.
    pub fn axis_stabilizer(&self, axis: Axis) -> Option<ConjugateCoset> {
        let (conjugating_element, orbit_index) = self.axis_undeorbiters[axis];
        let mut subgroup_solver_guard = self.axis_orbits[orbit_index].subgroup_solver.lock();
        ConjugateSubgroupConstraintSolver::new(conjugating_element, &mut subgroup_solver_guard)
            .solve(hypergroup::ConstraintSet::EMPTY)
    }

    /// Returns the order of the unit twist on an axis, or `None` if the axis
    /// does not have a unit twist.
    ///
    /// Typically, only axes on 3D puzzles have unit twists.
    pub fn unit_twist_order(&self, axis: Axis) -> Option<NonZeroI32> {
        let (_, orbit_index) = self.axis_undeorbiters[axis];
        self.axis_orbits[orbit_index]
            .stabilizer_twists
            .iter()
            .find(|(named_point_set, _unit_twist, _)| named_point_set.is_empty())
            .map(|(_named_point_set, unit_twist, _)| unit_twist.order)
    }

    fn constraints_from_notation(
        &self,
        notation_constraint_set: &hypuz_notation::ConstraintSet,
    ) -> Result<hypergroup::ConstraintSet<NamedPoint>, TwistError> {
        let name_to_id = |name: &Str| {
            self.named_point_names
                .lookup(name)
                .ok_or_else(|| TwistError::UnknownNamedPoint(name.clone()))
        };
        notation_constraint_set
            .constraints
            .iter()
            .map(|notation_constraint| -> Result<SmallVec<[_; 2]>, _> {
                Ok(match notation_constraint {
                    hypuz_notation::Constraint::FromTo([a, b]) => {
                        smallvec![[name_to_id(a)?, name_to_id(b)?].into()] // a -> b
                    }
                    hypuz_notation::Constraint::Swap([a, b]) => smallvec![
                        [name_to_id(a)?, name_to_id(b)?].into(), // a -> b
                        [name_to_id(b)?, name_to_id(a)?].into(), // b -> a
                    ],
                    hypuz_notation::Constraint::Fix(f) => {
                        smallvec![[name_to_id(f)?; 2].into()] // f -> f
                    }
                })
            })
            .flatten_ok()
            .try_collect()
    }

    fn constraints_to_notation(
        &self,
        hypergroup_constraint_set: hypergroup::ConstraintSet<NamedPoint>,
    ) -> hypuz_notation::ConstraintSet {
        hypergroup_constraint_set
            .iter()
            .map(|hypergroup_constraint| {
                hypuz_notation::Constraint::from((
                    &self.named_point_names[hypergroup_constraint.from],
                    &self.named_point_names[hypergroup_constraint.to],
                ))
            })
            .collect()
    }
}

/// Axis orbit data.
#[derive(Debug)]
pub struct SymmetricTwistSystemAxisOrbit {
    /// First axis in the orbit.
    pub first: Axis,
    /// Number of axes in the orbit.
    pub len: usize,
    /// Constraint solver for the stabilizer subgroup with respect to the axis.
    pub subgroup_solver: Mutex<SubgroupConstraintSolver<NamedPoint>>,
    /// Map from stabilizer twist family to unique minimal clockwise twist and
    /// gizmo pole distance. This only includes twists for the first axis in the
    /// orbit.
    ///
    /// The gizmo pole distance is only relevant in 4D, and is only needed when
    /// initially building the puzzle.
    pub stabilizer_twists: Vec<(NamedPointSet, UniqueMinimalClockwiseGenerator, Float)>,
    /// Jumbling data, if this axis jumbles.
    pub jumble_data: Option<AxisOrbitJumbleData>,
}

impl SymmetricTwistSystemAxisOrbit {
    /// Returns an iterator over all the axes in the orbit.
    pub fn axes(&self) -> TypedIndexIter<Axis> {
        let start = self.first.to_index();
        Axis::iter_range_clamped(start..start + self.len)
    }
}

/// Unique minimal clockwise generator for a cyclic subgroup.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct UniqueMinimalClockwiseGenerator {
    /// Unique generator for the subgroup that is the smallest clockwise
    /// rotation.
    pub element: GroupElementId,
    /// [Order] of the group element.
    ///
    /// This is always strictly positive, but is stored as `i32` for ease of use
    /// with [`Multiplier`].
    ///
    /// [order]: https://en.wikipedia.org/wiki/Order_(group_theory)
    pub order: NonZeroI32,
}

impl UniqueMinimalClockwiseGenerator {
    /// Constructs a unique minimal clockwise generator struct using the given
    /// element, inferring the order of the group.
    pub fn new(group: &hypergroup::Group, element: GroupElementId) -> Self {
        Self {
            element,
            order: NonZeroI32::new(group.period(element) as i32)
                .expect("group element period is zero"),
        }
    }
}

#[derive(thiserror::Error, Debug, Clone)]
pub enum TwistError {
    #[error("unknown axis: {0:?}")]
    UnknownAxis(Str),
    #[error("unknown named point: {0:?}")]
    UnknownNamedPoint(Str),
    #[error("unsatisfiable constraints")]
    UnsatisfiableConstraints,
    #[error("constraints require reflection")]
    Reflection,
    #[error("underconstrained ({coset_size} possibilities)")]
    Underconstrained { coset_size: usize },
    #[error("ambiguous 180° rotation")]
    Ambiguous180,
    #[error("direct rotation does not exist")]
    DirectRotationDoesNotExist,
    #[error("direct rotation does not preserve axis")]
    DirectRotationDoesNotFixAxis,
    #[error("constraints force identity")]
    Identity,
}

/// Additional axis orbit data for jumbling axes.
///
/// # Examples
///
/// ## Bagua cube
///
/// ```
/// AxisOrbitJumbleData {
///     stops: PerJumbleStop::from_iter([
///         JumbleStopInfo {
///             angle: 0.0,
///             to_doctrinaire_prefer_next: None,
///             to_doctrinaire_prefer_prev: None,
///         },
///     ]),
/// };
/// ```
///
/// ## Deep-Cut FT Icosahedron (Radiolarian )
///
/// ```
/// ```
///
/// ## Trianglular bipyramid
///
/// ```
/// ```
#[derive(Debug, Clone)]
pub struct AxisOrbitJumbleData {
    /// List of unit clockwise jumble transforms, including the doctrinaire
    /// twist. Transforms are sorted from largest angle to smallest. The first
    /// element is always the doctrinaire unit twist transform.
    pub transforms: Vec<JumbleTransform>,
    /// Jumble stop angles, in the range 0..τ.
    pub stops: PerJumbleStop<JumbleStopInfo>,
    /// Doctrinaire transform, if there is one.
    pub doctrinaire_transform: Option<JumbleTransform>,
}

impl AxisOrbitJumbleData {
    pub(crate) fn new(
        transforms: Vec<JumbleTransform>,
        stops: impl IntoIterator<Item = Float>,
    ) -> Result<Self> {
        let stops = stops
            .into_iter()
            .map(|angle| hypermath::util::canonicalize_angle(angle))
            .sorted_by(Float::total_cmp)
            .dedup_by(|a, b| APPROX.eq(a, b))
            .map(|angle| JumbleStopInfo::new(&transforms, angle))
            .try_collect()?;

        let doctrinaire_transform = transforms.iter().find(|tf| tf.is_doctrinaire).cloned();

        Ok(Self {
            transforms,
            stops,
            doctrinaire_transform,
        })
    }

    fn doctrinaire_unit_transform(&self) -> Option<&JumbleTransform> {
        self.transforms.iter().find(|tf| tf.is_doctrinaire)
    }

    /// Returns the angle of the doctrinaire twist, or τ if there is none.
    fn doctrinaire_angle(&self) -> Float {
        self.doctrinaire_transform
            .as_ref()
            .map(|tf| tf.angle)
            .unwrap_or(TAU)
    }

    #[cfg(test)]
    fn doctrinaire_order(&self) -> usize {
        hypermath::util::to_integer(TAU / self.doctrinaire_angle())
            .expect("doctrinaire twist is not a simple fraction of TAU")
    }

    /// Returns the nearest jumbling stop from an absolute angle.
    pub fn factor_nearest(&self, angle: Float) -> JumbleStop {
        // linear search is fastest because the list is always short
        self.stops
            .iter()
            .min_by_float_key(|(_, stop_info)| (stop_info.angle - angle).abs())
            .map(|(stop, _)| stop)
            .expect("empty jumble stop list")
    }

    /// Returns a jumbling stop from an absolute angle, or `None` if there is no
    /// jumbling stop at that approximate angle.
    pub fn factor_exact(&self, angle: Float) -> Option<JumbleStop> {
        let angle = hypermath::util::canonicalize_angle(angle);
        // linear search is fastest because the list is always short
        self.stops
            .find(|_, stop_info| APPROX.eq(stop_info.angle, angle))
    }

    pub fn notation_from_stop_to_stop(
        &self,
        start: JumbleStop,
        end: JumbleStop,
        preferred_sign: Option<Sign>,
    ) -> Result<SmallVec<[JumbleTransform; 3]>> {
        let angle_delta = hypermath::util::minimize_angle(
            self.stops[end].angle - self.stops[start].angle,
            preferred_sign,
        );

        if APPROX.eq_zero(angle_delta) {
            return Ok(smallvec![]);
        }

        if let Ok(tf) = find_simple_transform_for_angle(&self.transforms, angle_delta) {
            return JumbleTransform::simplify_seq([tf]);
        }

        // Fallback: `start` -> doctrinaire position near `start` -> doctrinaire position near `end` -> `end`
        let stop1 = &self.stops[start];
        let stop2 = &self.stops[end];
        let (tf1, tf2) = if angle_delta > 0.0 {
            (
                stop1.to_doctrinaire_prefer_next.clone(),
                stop2.to_doctrinaire_prefer_prev.rev()?,
            )
        } else {
            (
                stop1.to_doctrinaire_prefer_prev.clone(),
                stop2.to_doctrinaire_prefer_next.rev()?,
            )
        };
        let doctrinaire_multiplier = hypermath::util::to_integer(
            hypermath::util::minimize_angle(angle_delta - tf1.angle - tf2.angle, preferred_sign)
                / self.doctrinaire_angle(),
        )
        .ok_or_eyre("no doctrinaire bridge between jumbling stops")?;
        if doctrinaire_multiplier != 0
            && let Some(doctrinaire_tf) = self.doctrinaire_unit_transform()
        {
            let doctrinaire_bridge = doctrinaire_tf.multiply(doctrinaire_multiplier)?;
            JumbleTransform::simplify_seq([tf1, doctrinaire_bridge, tf2])
        } else {
            JumbleTransform::simplify_seq([tf1, tf2])
        }
    }

    pub fn adjacent_stop(&self, stop: JumbleStop, sign: Sign) -> JumbleStop {
        let mut i = stop.0 as i64 + self.stops.len() as i64;
        i += sign.to_num::<i64>();
        JumbleStop((i % self.stops.len() as i64) as u16)
    }

    pub fn suffix_to_angle(&self, suffix: JumbleSuffix) -> Option<Float> {
        self.transforms
            .iter()
            .find(|tf| tf.suffix == Some(suffix))
            .map(|tf| tf.angle)
    }
}

fn find_simple_transform_for_angle(
    transforms: &[JumbleTransform],
    angle: Float,
) -> Result<JumbleTransform, NoSimpleTransform> {
    if APPROX.eq_zero(angle) {
        return Ok(JumbleTransform::EMPTY);
    }

    // Try to use a doctrinaire or jumbling move if possible, preferring
    // larger angles
    for tf in transforms {
        if let Some(i) = hypermath::util::to_integer(angle / tf.angle)
            && let Ok(multiplied_tf) = tf.multiply(i)
        {
            return Ok(multiplied_tf);
        }
    }

    Err(NoSimpleTransform(angle).into())
}

#[derive(thiserror::Error, Debug, Default, Copy, Clone)]
#[error("no simple transform for angle {0} rad = {deg}°", deg = .0.to_degrees())]
struct NoSimpleTransform(Float);

#[derive(Debug, Clone)]
pub struct JumbleStopInfo {
    /// Angle offset of the jumble stop.
    pub angle: Float,
    /// Single move to a nearby doctrinaire position.
    ///
    /// If there is a single move to the next jumbling stop _and_ a single move
    /// to the previous one, then this is the move to the **next** jumbling
    /// stop.
    ///
    /// For this is a doctrinaire position, this is [`JumbleTransform::EMPTY`].
    to_doctrinaire_prefer_next: JumbleTransform,
    /// Single move to a nearby doctrinaire position.
    ///
    /// If there is a single move to the next jumbling stop _and_ a single move
    /// to the previous one, then this is the move to the **next** jumbling
    /// stop.
    ///
    /// For this is a doctrinaire position, this is [`JumbleTransform::EMPTY`].
    to_doctrinaire_prefer_prev: JumbleTransform,
}

impl JumbleStopInfo {
    /// Constructs a jumble stop representing a doctrinaire position.
    ///
    /// Every axis orbit must have at least one of these at angle 0 as the first
    /// element in the list of jumble stops.
    pub fn doctrinaire(angle: Float) -> Self {
        Self {
            angle,
            to_doctrinaire_prefer_next: JumbleTransform::EMPTY,
            to_doctrinaire_prefer_prev: JumbleTransform::EMPTY,
        }
    }

    pub fn new(transforms: &[JumbleTransform], angle: Float) -> Result<Self> {
        if let Some(duplicate) = transforms.iter().map(|t| t.suffix).duplicates().next() {
            bail!(
                "duplicate jumble suffix {:?}",
                duplicate.map(|s| s.to_string()).unwrap_or_default(),
            );
        }

        let doctrinaire_angle = transforms
            .iter()
            .find(|tf| tf.is_doctrinaire)
            .map(|tf| tf.angle.abs())
            .unwrap_or(TAU);
        let residue = hypermath::util::approx_rem_euclid(angle, doctrinaire_angle.abs());
        if !residue.is_finite() {
            bail!("invalid jumbling angle {residue} derived from {angle}");
        } else if APPROX.eq_zero(residue) {
            Ok(Self::doctrinaire(angle))
        } else {
            let to_next = find_simple_transform_for_angle(transforms, doctrinaire_angle - residue);
            let to_prev = find_simple_transform_for_angle(transforms, -residue);
            Ok(Self {
                angle,
                to_doctrinaire_prefer_next: to_next.clone().or_else(|_| to_prev.clone())?,
                to_doctrinaire_prefer_prev: to_prev.or(to_next)?,
            })
        }
    }
}

/// Jumbling move, irrespective of axis or layers.
#[derive(Debug, Copy, Clone)]
pub struct JumbleTransform {
    /// Jumble suffix, which is `None` to indicate a doctrinaire move.
    pub suffix: Option<JumbleSuffix>,
    /// Twist multiplier.
    pub multiplier: Multiplier,
    /// Total **clockwise** angle for the transform. For a unit twist, this
    /// value is constrained to the range -π..=π. For a non-unit twist, this
    /// value is _not_ constrained to any particular range.
    ///
    /// This is ignored for equality comparing.
    pub angle: Float,
    /// Whether the transform is doctrinaire.
    pub is_doctrinaire: bool,
}
#[cfg(test)]
impl Eq for JumbleTransform {}
#[cfg(test)]
impl PartialEq for JumbleTransform {
    fn eq(&self, other: &Self) -> bool {
        self.suffix == other.suffix && self.multiplier == other.multiplier
    }
}
impl JumbleTransform {
    /// Empty transform.
    pub const EMPTY: Self = Self {
        suffix: None,
        multiplier: Multiplier(0),
        angle: 0.0,
        is_doctrinaire: false,
    };

    /// Constructs a doctrinaire transform with the given order. The suffix is
    /// blank.
    pub fn new_unit_doctrinaire(order: usize) -> Self {
        Self {
            suffix: None,
            multiplier: Multiplier(1),
            angle: TAU / order as Float,
            is_doctrinaire: true,
        }
    }

    /// Constructs a jumbling transform with the given suffix and angle.
    pub fn new_unit_jumbling(suffix: JumbleSuffix, angle: Float) -> Self {
        Self {
            suffix: Some(suffix),
            multiplier: Multiplier(1),
            angle,
            is_doctrinaire: false,
        }
    }

    pub fn on_axis_with_layers(
        &self,
        layers: impl Into<LayerPrefix>,
        axis_name: &str,
        separator: &str,
    ) -> Move {
        let family = if let Some(suffix) = self.suffix {
            &format!("{axis_name}{separator}{suffix}")
        } else {
            axis_name
        };
        Move::new(layers, family, None, self.multiplier)
    }

    /// Multiplies the twist by a multiplier. Returns an error on overflow.
    pub fn multiply(&self, n: i32) -> Result<Self> {
        Ok(Self {
            suffix: self.suffix.clone(),
            multiplier: Multiplier(self.multiplier.0.checked_mul(n).ok_or_eyre("overflow")?),
            angle: self.angle * n as Float,
            is_doctrinaire: self.is_doctrinaire,
        })
    }

    pub fn rev(&self) -> Result<Self> {
        self.multiply(-1)
    }

    fn simplify_seq(seq: impl IntoIterator<Item = Self>) -> Result<SmallVec<[Self; 3]>> {
        let mut ret: SmallVec<[Self; _]> = smallvec![];
        for item in seq {
            if APPROX.eq_zero(hypermath::util::canonicalize_angle(item.angle)) {
            } else if let Some(prev) = ret.last_mut()
                && prev.suffix == item.suffix
            {
                prev.multiplier.0 = i32::checked_add(prev.multiplier.0, item.multiplier.0)
                    .ok_or_eyre("overflow")?;
                prev.angle += item.angle;
            } else {
                ret.push(item);
            }
        }
        Ok(ret)
    }
}

#[cfg(test)]
mod tests {
    use std::f64::consts::PI;

    use super::*;

    #[test]
    fn test_jumble_stops() {
        // Rhombic dodecahedron
        let d = TAU / 2.0;
        let j = (1.0 / 3.0_f64).acos();
        let transforms = vec![
            JumbleTransform::new_unit_doctrinaire(2),
            JumbleTransform::new_unit_jumbling(JumbleSuffix::J(None), j),
        ];
        let stops = (0..2_i32)
            .map(|i| d * i as Float)
            .flat_map(|base| [base, base + j, base - j]);
        test_jumble_configuration(&AxisOrbitJumbleData::new(transforms, stops).unwrap());

        // Icosahedron
        let d = TAU / 3.0;
        let j = d - (1.0 / 4.0_f64).acos();
        let transforms = vec![
            JumbleTransform::new_unit_doctrinaire(3),
            JumbleTransform::new_unit_jumbling(JumbleSuffix::J(None), j),
        ];
        let stops = (0..3_i32)
            .map(|i| d * i as Float)
            .flat_map(|base| [base, base + j, base - j]);
        test_jumble_configuration(&AxisOrbitJumbleData::new(transforms, stops).unwrap());

        // Bagua cube
        let d = TAU / 4.0;
        let h = d / 2.0;
        let transforms = vec![
            JumbleTransform::new_unit_doctrinaire(4),
            JumbleTransform::new_unit_jumbling(JumbleSuffix::H, h),
        ];
        let stops = (0..4_i32)
            .map(|i| d * i as Float)
            .flat_map(|base| [base, base + h, base - h]); // redundancy should get filtered out
        let bagua_cube = AxisOrbitJumbleData::new(transforms, stops).unwrap();
        test_jumble_configuration(&bagua_cube);

        // Test bagua cube notation
        for (i, preferred_sign, expected) in [
            (0, Sign::Pos, None),
            (1, Sign::Pos, Some((Some(JumbleSuffix::H), 1))),
            (2, Sign::Pos, Some((None, 1))),
            (3, Sign::Pos, Some((Some(JumbleSuffix::H), 3))),
            (4, Sign::Pos, Some((None, 2))),
            (5, Sign::Pos, Some((Some(JumbleSuffix::H), -3))),
            (6, Sign::Pos, Some((None, -1))),
            (7, Sign::Pos, Some((Some(JumbleSuffix::H), -1))),
            (0, Sign::Neg, None),
            (1, Sign::Neg, Some((Some(JumbleSuffix::H), 1))),
            (2, Sign::Neg, Some((None, 1))),
            (3, Sign::Neg, Some((Some(JumbleSuffix::H), 3))),
            (4, Sign::Neg, Some((None, -2))),
            (5, Sign::Neg, Some((Some(JumbleSuffix::H), -3))),
            (6, Sign::Neg, Some((None, -1))),
            (7, Sign::Neg, Some((Some(JumbleSuffix::H), -1))),
        ] {
            assert_eq!(
                bagua_cube
                    .notation_from_stop_to_stop(JumbleStop(0), JumbleStop(i), Some(preferred_sign))
                    .unwrap()
                    .to_vec(),
                expected
                    .into_iter()
                    .map(|(suffix, multiplier)| JumbleTransform {
                        suffix: suffix.into(),
                        multiplier: Multiplier(multiplier),
                        angle: 0.0,            // ignored for comparison
                        is_doctrinaire: false, // ignored for comparison
                    })
                    .collect_vec(),
            );
        }

        // Hexagonal bipyramid (jumbling-only, multiple jumble stops)
        let ja = (-1.0 / 4.0_f64).acos();
        let jb = (-7.0 / 8.0_f64).acos();
        let transforms = vec![
            JumbleTransform::new_unit_jumbling(JumbleSuffix::J(Some(0.into())), ja),
            JumbleTransform::new_unit_jumbling(JumbleSuffix::J(Some(1.into())), jb),
        ];
        let stops = [0.0, ja, jb, -ja, -jb];
        test_jumble_configuration(&AxisOrbitJumbleData::new(transforms, stops).unwrap());
    }

    /// Asserts that the generated transform sequence for all possible start/end
    /// jumble stop pairs are valid; in particular, that every transform ends on
    /// a jumble stop.
    fn test_jumble_configuration(jumble_data: &AxisOrbitJumbleData) {
        for start in jumble_data.stops.iter_keys() {
            for end in jumble_data.stops.iter_keys() {
                for preferred_sign in [None, Some(Sign::Pos), Some(Sign::Neg)] {
                    let transform_seq = jumble_data
                        .notation_from_stop_to_stop(start, end, preferred_sign)
                        .expect("failed to find move seq");

                    let mut angle = jumble_data.stops[start].angle;
                    for transform in &transform_seq {
                        assert!(
                            jumble_data.factor_exact(angle).is_some(),
                            "transform sequence lands on a place that isn't a jumble stop",
                        );
                        angle += transform.angle;
                    }

                    let net_angle = transform_seq.iter().map(|tf| tf.angle).sum();
                    match preferred_sign {
                        Some(Sign::Pos) => assert!(APPROX.ne(net_angle, -PI)),
                        Some(Sign::Neg) => assert!(APPROX.ne(net_angle, PI)),
                        None => (),
                    }

                    assert_eq!(
                        jumble_data.factor_exact(angle),
                        Some(end),
                        "transform sequence did not end at the right place",
                    );
                }
            }
        }
    }
}
