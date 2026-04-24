use std::collections::HashMap;
use std::num::NonZeroI32;
use std::sync::Arc;

use eyre::{Context, OptionExt, Result, bail, eyre};
use hypergroup::{
    ConjugateCoset, ConjugateSubgroupConstraintSolver, ConstraintSolver, Group, GroupAction,
    SubgroupAction, SubgroupConstraintSolver,
};
use hypermath::pga::Motor;
use hypermath::prelude::*;
use hyperpuzzle_core::TwistSystemEngineData;
use hyperpuzzle_core::group::{GroupElementId, IsometryGroup};
use hyperpuzzle_core::prelude::*;
use hypuz_notation::{Str, Transform};
use hypuz_util::{FloatMinMaxByIteratorExt, FloatMinMaxIteratorExt};
use itertools::Itertools;
use parking_lot::Mutex;
use rand::{Rng, RngExt};
use smallvec::{SmallVec, smallvec};

use crate::{NamedPoint, NamedPointSet, PerNamedPoint, StabilizerFamily};

struct AxisConstraintSolver {
    deorbiter: GroupElementId,
    solver: Arc<Mutex<ConstraintSolver<NamedPoint>>>,
}
impl AxisConstraintSolver {}

/// Simulation data for a symmetric puzzle.
///
/// This type is relatively cheap to clone.
#[derive(Debug, Clone)]
pub struct SymmetricTwistSystemEngineData {
    /// Axis system.
    pub axes: Arc<AxisSystem>,
    /// Physical location of each axis, for constructing simple direct rotations
    /// from one axis to another.
    pub axis_vectors: Arc<PerAxis<Vector>>,
    /// Grip group, which is the symmetry group of the axis system.
    pub group: IsometryGroup,
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

    /// Action of the grip group on the named points.
    pub named_point_action: GroupAction<NamedPoint>,
    /// Named point names.
    pub named_point_names: Arc<NameSpecBiMap<NamedPoint>>,
    /// Physical location of each named point, for constructing simple direct
    /// rotations from one named point to another.
    pub named_point_vectors: Arc<PerNamedPoint<Vector>>,
}

impl TwistSystemEngineData for SymmetricTwistSystemEngineData {}

impl SymmetricTwistSystemEngineData {
    /// Returns the number of dimensions of the space containing the puzzle.
    pub fn ndim(&self) -> u8 {
        self.group.ndim()
    }

    /// Returns the motor for a twist.
    pub fn twist_motor(&self, twist: &Move) -> Result<Motor> {
        let (_axis, transform) = self.resolve_twist_transform(&twist.transform)?;
        let multiplied_element = self.group.powi(transform, twist.multiplier.0);
        Ok(self.group.motor(multiplied_element))
    }

    /// Resolves a twist to an axis and a group element.
    pub fn resolve_twist(&self, twist: &Move) -> Result<(Axis, GroupElementId), TwistError> {
        let (axis, element) = self.resolve_twist_transform(&twist.transform)?;
        Ok((axis, self.group.powi(element, twist.multiplier.0)))
    }

    /// Resolves a twist transform to an axis and a group element.
    pub fn resolve_twist_transform(
        &self,
        transform: &Transform,
    ) -> Result<(Axis, GroupElementId), TwistError> {
        let Some(axis) = self.axes.names.id_from_name(&transform.family) else {
            let separator = '_'; // TODO: correct number of underscores (maybe none)
            if let Some((primary_axis_str, secondary_axes_str)) =
                transform.family.split_once(separator)
                && let Some(primary) = self.axes.names.id_from_name(primary_axis_str)
                && let Some(secondary) = secondary_axes_str
                    .split(separator)
                    .map(|s| self.named_point_names.id_from_name(s))
                    .collect::<Option<_>>()
                    .and_then(|axes| NamedPointSet::new(axes).ok())
                && let Some(unit_twist) =
                    self.resolve_stabilizer_twist_transform(StabilizerFamily { primary, secondary })
            {
                return Ok((primary, unit_twist.element)); // 4D stabilizer notation
            } else {
                return Err(TwistError::UnknownAxis(transform.family.clone()));
            }
        };

        if transform.constraints.is_none()
            && let Some(unit_twist) = self.resolve_stabilizer_twist_transform(StabilizerFamily {
                primary: axis,
                secondary: NamedPointSet::EMPTY,
            })
        {
            return Ok((axis, unit_twist.element)); // 3D stabilizer notation
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

        Ok((axis, element))
    }

    /// Resolves a stabilizer family to a unique minimal clockwise twist.
    pub fn resolve_stabilizer_twist_transform(
        &self,
        stabilizer_family: StabilizerFamily,
    ) -> Option<UniqueMinimalClockwiseGenerator> {
        let (conjugating_element, orbit_index) = self.axis_undeorbiters[stabilizer_family.primary];
        let axis_orbit = &self.axis_orbits[orbit_index];
        let mut subgroup_solver = axis_orbit.subgroup_solver.lock();
        let mut solver =
            ConjugateSubgroupConstraintSolver::new(conjugating_element, &mut subgroup_solver);

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

        Some(self.constraints_to_notation(
            solver.constraints_for_element(hypergroup::ConstraintSet::EMPTY, random_rotation)?,
        ))
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
                .id_from_name(name)
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
    /// Constraint solver for the stabilizer subgroup with respect to the axis.
    pub subgroup_solver: Mutex<SubgroupConstraintSolver<NamedPoint>>,
    /// Map from stabilizer twist family to unique minimal clockwise twist and
    /// gizmo pole distance.
    ///
    /// The gizmo pole distance is only relevant in 4D, and is only needed when
    /// initially building the puzzle.
    pub stabilizer_twists: Vec<(NamedPointSet, UniqueMinimalClockwiseGenerator, Float)>,
}

/// Unique minimal clockwise generator for a cyclic subgroup.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct UniqueMinimalClockwiseGenerator {
    /// Unique generator for the subgroup that is the smallest clockwise
    /// rotation.
    pub element: GroupElementId,
    /// [Order] of the group element.
    ///
    /// [order]: https://en.wikipedia.org/wiki/Order_(group_theory)
    pub order: NonZeroI32,
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
