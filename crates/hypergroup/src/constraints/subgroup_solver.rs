use hypuz_util::ti::TypedIndex;

use super::ConstraintSolver;
use crate::{
    ConjugateCoset, ConstraintSet, Group, GroupAction, GroupElementId, Subgroup, SubgroupAction,
};

/// [`ConstraintSolver`] that is pre-constrained to a particular subgroup.
///
/// See also: [`crate::ConjugateSubgroupConstraintSolver`].
#[derive(Debug)]
pub struct SubgroupConstraintSolver<P> {
    action: SubgroupAction<P>,
    solver: ConstraintSolver<P>,
}

impl<P: TypedIndex> SubgroupConstraintSolver<P> {
    /// Constructs a new constraint solver.
    pub fn new(action: SubgroupAction<P>) -> Self {
        let solver = ConstraintSolver::new(action.subgroup_action.clone());
        Self { action, solver }
    }

    /// Returns the overgroup, which is the original group that this is a
    /// subgroup of.
    pub fn overgroup(&self) -> &Group {
        self.action.overgroup()
    }
    /// Returns the action of the overgroup.
    pub fn overgroup_action(&self) -> &GroupAction<P> {
        self.action.overgroup_action()
    }
    /// Returns the action of the subgroup.
    pub fn subgroup_action(&self) -> &SubgroupAction<P> {
        &self.action
    }

    /// Returns the coset with the subgroup satisfying a set of constraints, or
    /// `None` if there is no such coset.
    pub fn solve(&mut self, constraint_set: &ConstraintSet<P>) -> Option<ConjugateCoset> {
        let ConjugateCoset { subgroup, lhs, rhs } = self.solver.solve(constraint_set)?;
        let Subgroup {
            overgroup: _, // actually self.action.subgroup_action.group()
            element_count,
            generators,
        } = subgroup;

        let subgroup = Subgroup {
            overgroup: self.overgroup().clone(),
            element_count,
            generators: generators
                .into_iter()
                .map(|g| self.action.subgroup_to_overgroup(g))
                .collect(),
        };
        Some(ConjugateCoset {
            subgroup,
            lhs: self.action.subgroup_to_overgroup(lhs),
            rhs: self.action.subgroup_to_overgroup(rhs),
        })
    }

    /// Selects an random element deterministically and uniformly from the
    /// subgroup that satisfies a set of constraints and returns a set of
    /// constraints that precisely specifies it.
    ///
    /// `random_index` must return a deterministic random integer in the range
    /// `0..n`. It will never be called with `n == 0`.
    ///
    /// The deterministic random output is guaranteed to be the same as would be
    /// returned by [`ConstraintSolver::select()`] in the larger group with an
    /// equivalent set of constraints.
    pub fn select(
        &mut self,
        constraint_set: &ConstraintSet<P>,
        random_index: impl FnMut(usize) -> usize,
    ) -> Option<(ConstraintSet<P>, GroupElementId)> {
        let (constraint_set, element) = self.solver.select(constraint_set, random_index)?;
        Some((constraint_set, self.action.subgroup_to_overgroup(element)))
    }

    /// Returns a canonical set of constraints that uniquely specifies an
    /// element within the subgroup, assuming the base constraints have already
    /// been applied.
    ///
    /// Returns `None` if `base_constraints` is unsatisfiable or if `element` is
    /// not in the subgroup.
    ///
    /// Panics in debug mode if `element` does not satisfy `base_constraints`.
    pub fn constraints_for_element(
        &mut self,
        base_constraints: &ConstraintSet<P>,
        element: GroupElementId,
    ) -> Option<ConstraintSet<P>> {
        self.solver.constraints_for_element(
            base_constraints,
            self.action.overgroup_to_subgroup(element)?,
        )
    }
}
