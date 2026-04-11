use std::collections::HashMap;
use std::num::NonZeroI32;
use std::sync::Arc;

use eyre::Result;
use hypergroup::{ConjugateCoset, ConstraintSolver, GroupAction};
use hypermath::pga::Motor;
use hypermath::prelude::*;
use hyperpuzzle_core::TwistSystemEngineData;
use hyperpuzzle_core::group::{GroupElementId, IsometryGroup};
use hyperpuzzle_core::prelude::*;
use hypuz_notation::{Str, Transform};
use hypuz_util::FloatMinMaxByIteratorExt;
use itertools::Itertools;
use parking_lot::Mutex;
use rand::{Rng, RngExt};
use smallvec::{SmallVec, smallvec};

use crate::{StabilizableAxisSet, StabilizerFamily};

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
    pub group_action: GroupAction<Axis>,
    /// Constraint solver based on the grip group.
    pub constraint_solver: Arc<Mutex<ConstraintSolver<Axis>>>,

    /// Unique minimal clockwise twist for each axis.
    ///
    /// This only exists in 3D.
    pub axis_unit_twists: Arc<PerAxis<Option<UniqueMinimalClockwiseGenerator>>>,

    /// Map from stablizer twist family to unique minimal clockwise twist.
    pub stabilizer_twists: Arc<Vec<(StabilizerFamily, UniqueMinimalClockwiseGenerator)>>,
    // TODO: consider unifying `axis_unit_twists` with `stabilizer_twists`
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
                    .map(|s| self.axes.names.id_from_name(s))
                    .collect::<Option<_>>()
                    .and_then(|axes| StabilizableAxisSet::new(axes).ok())
                && let Some(group_element_id) =
                    self.resolve_stabilizer_twist_transform(StabilizerFamily { primary, secondary })
            {
                return Ok((primary, group_element_id.element));
            } else {
                return Err(TwistError::UnknownAxis(transform.family.clone()));
            }
        };

        if transform.constraints.is_none()
            && let Some(unit_twist) = self.axis_unit_twists[axis]
        {
            return Ok((axis, unit_twist.element));
        }

        let mut constraints = smallvec![hypergroup::Constraint::fix(axis)];
        if let Some(notation_constraints) = &transform.constraints {
            for c in notation_constraints {
                constraints.extend(self.notation_constraint_to_hypergroup_constraint(c)?);
            }
        }
        let constraint_set = hypergroup::ConstraintSet { constraints };

        let coset = self
            .constraint_solver
            .lock()
            .solve(&constraint_set)
            .ok_or(TwistError::UnsatisfiableConstraints)?;

        let rotation_count = self.count_rotations_in_coset(&coset);

        let element = if rotation_count == 0 {
            return Err(TwistError::UnsatisfiableConstraints);
        } else if rotation_count == 1
            && let Ok(unambiguous_rotation_in_coset) = self.rotations_in_coset(&coset).exactly_one()
        {
            unambiguous_rotation_in_coset
        } else if let Some((_fixed_axis_constraint, single_constraint)) =
            constraint_set.constraints.iter().collect_tuple()
        {
            let direct_rotation = Motor::rotation(
                &self.axis_vectors[single_constraint.from],
                &self.axis_vectors[single_constraint.to],
            )
            .ok_or(TwistError::Ambiguous180)?;
            let element = self
                .group
                .element_from_motor(&direct_rotation)
                .ok_or(TwistError::DirectRotationDoesNotExist)?;

            if self.group_action.act(element, axis) == axis {
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

    pub fn resolve_stabilizer_twist_transform(
        &self,
        stabilizer_family: StabilizerFamily,
    ) -> Option<UniqueMinimalClockwiseGenerator> {
        for (candidate, unit_twist) in &*self.stabilizer_twists {
            if stabilizer_family.secondary.len() == candidate.secondary.len()
                && let Some(coset) =
                    self.constraint_solver
                        .lock()
                        .solve(&hypergroup::ConstraintSet::from_iter(
                            std::iter::zip(
                                candidate.iter_flatten(),
                                stabilizer_family.iter_flatten(),
                            )
                            .map(|(from, to)| hypergroup::Constraint { from, to }),
                        ))
            {
                // The coset stabilizes the twist transform, so it doesn't
                // matter which element we take from it.
                let coset_representative = coset.arbitrary_element();
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
        let mut solver = self.constraint_solver.lock();
        let fixed_axis_constraint_set =
            hypergroup::ConstraintSet::from_iter([hypergroup::Constraint::fix(axis)]);

        let coset = solver.solve(&fixed_axis_constraint_set)?;
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
                    solver.select(&fixed_axis_constraint_set, |n| rng.random_range(0..n))
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

        Some(hypuz_notation::ConstraintSet {
            constraints: solver
                .constraints_for_element(&fixed_axis_constraint_set, random_rotation)?
                .into_iter()
                .map(|c| self.hypergroup_constraint_to_notation_constraint(c))
                .collect(),
        })
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
        self.constraint_solver
            .lock()
            .solve(&hypergroup::ConstraintSet {
                constraints: smallvec![hypergroup::Constraint::fix(axis)],
            })
    }

    fn notation_constraint_to_hypergroup_constraint(
        &self,
        notation_constraint: &hypuz_notation::Constraint,
    ) -> Result<SmallVec<[hypergroup::Constraint<Axis>; 2]>, TwistError> {
        let name_to_id = |name: &Str| {
            self.axes
                .names
                .id_from_name(name)
                .ok_or_else(|| TwistError::UnknownAxis(name.clone()))
        };
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
    }

    fn hypergroup_constraint_to_notation_constraint(
        &self,
        hypergroup_constraint: hypergroup::Constraint<Axis>,
    ) -> hypuz_notation::Constraint {
        hypuz_notation::Constraint::from((
            &self.axes.names[hypergroup_constraint.from],
            &self.axes.names[hypergroup_constraint.to],
        ))
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
