use hypuz_notation::{Str, Transform};
use itertools::Itertools;
use parking_lot::Mutex;
use smallvec::{SmallVec, smallvec};
use std::sync::Arc;

use eyre::Result;
use hypergroup::{ConstraintSolver, GroupAction};
use hypermath::pga::Motor;
use hypermath::prelude::*;
use hyperpuzzle_core::TwistSystemEngineData;
use hyperpuzzle_core::group::{GroupElementId, IsometryGroup};
use hyperpuzzle_core::prelude::*;

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
}

impl TwistSystemEngineData for SymmetricTwistSystemEngineData {}

impl SymmetricTwistSystemEngineData {
    /// Returns the number of dimensions of the space containing the puzzle.
    pub fn ndim(&self) -> u8 {
        self.group.ndim()
    }

    pub fn twist_motor(&self, twist: &Move) -> Result<Motor> {
        let (_axis, transform) = self.resolve_twist_transform(&twist.transform)?;
        let multiplied_element = self.group.powi(transform, twist.multiplier.0);
        Ok(self.group.motor(multiplied_element))
    }

    pub fn resolve_twist(&self, twist: &Move) -> Result<(Axis, GroupElementId), TwistError> {
        let (axis, element) = self.resolve_twist_transform(&twist.transform)?;
        Ok((axis, self.group.powi(element, twist.multiplier.0)))
    }

    pub fn resolve_twist_transform(
        &self,
        transform: &Transform,
    ) -> Result<(Axis, GroupElementId), TwistError> {
        let Some(axis) = self.axes.names.id_from_name(&transform.family) else {
            return Err(TwistError::UnknownAxis(transform.family.clone()));
        };

        let mut constraints = smallvec![hypergroup::Constraint::fix(axis)];
        if let Some(notation_constraints) = &transform.constraints {
            for c in notation_constraints {
                constraints.extend(self.notation_constraint_to_hypergroup_constraint(c)?);
            }
        }
        let constraint_set = hypergroup::ConstraintSet { constraints };
        dbg!(&constraint_set);

        let coset = self
            .constraint_solver
            .lock()
            .solve(&constraint_set)
            .ok_or(TwistError::UnsatisfiableConstraints)?;

        // Does the coset have any reflections and/or rotations?
        let (has_refl, has_rot) = if coset
            .subgroup_generators
            .iter()
            .any(|g| self.group.motor(*g).is_reflection())
        {
            (true, true) // reflections and rotations
        } else if self.group.motor(coset.lhs).is_reflection() {
            (true, false) // reflections only
        } else {
            (false, true) // rotations only
        };

        let rotation_count = if has_rot {
            if has_refl {
                coset.element_count / 2
            } else {
                coset.element_count
            }
        } else {
            0
        };

        let element = if rotation_count == 1
            && let Some(unambiguous_rotation_in_coset) = coset
                .elements()
                .into_iter()
                .filter(|&e| !self.group.motor(e).is_reflection())
                .exactly_one()
                .ok()
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
    #[error("constraint force identity")]
    Identity,
}
