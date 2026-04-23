use hypuz_util::ti::TypedIndex;

use super::SubgroupConstraintSolver;
use crate::{
    ConjugateCoset, Constraint, ConstraintSet, Group, GroupAction, GroupElementId, SubgroupAction,
};

/// [`crate::ConstraintSolver`] that is pre-constrained to a particular
/// conjugate subgroup.
#[derive(Debug)]
pub struct ConjugateSubgroupConstraintSolver<'a, P> {
    /// Conjugating element.
    lhs: GroupElementId,
    subgroup_solver: &'a mut SubgroupConstraintSolver<P>,
    /// Inverse of conjugating element.
    rhs: GroupElementId,
}

impl<'a, P: TypedIndex> ConjugateSubgroupConstraintSolver<'a, P> {
    /// Constructs a constraint solver for the conjugate subgroup `x H x^-1`
    /// where `x` is the conjugating element and `H` is the subgroup.
    pub fn new(
        conjugating_element: GroupElementId,
        subgroup_solver: &'a mut SubgroupConstraintSolver<P>,
    ) -> Self {
        let inv_conjugating_element = subgroup_solver.overgroup().inverse(conjugating_element);
        Self {
            lhs: conjugating_element,
            subgroup_solver,
            rhs: inv_conjugating_element,
        }
    }

    /// Returns the conjugating element.
    pub fn lhs(&self) -> GroupElementId {
        self.lhs
    }

    /// Returns the inverse conjugating element.
    pub fn rhs(&self) -> GroupElementId {
        self.rhs
    }

    /// Returns the overgroup, which is the original group that this is a
    /// subgroup of.
    pub fn overgroup(&self) -> &Group {
        self.subgroup_solver.overgroup()
    }
    /// Returns the action of the overgroup.
    pub fn overgroup_action(&self) -> &GroupAction<P> {
        self.subgroup_solver.overgroup_action()
    }
    /// Returns the action of the original subgroup.
    pub fn subgroup_action(&self) -> &SubgroupAction<P> {
        self.subgroup_solver.subgroup_action()
    }
    /// Returns the solver for the (non-conjugated) subgroup.
    pub fn subgroup_solver(&mut self) -> &mut SubgroupConstraintSolver<P> {
        self.subgroup_solver
    }

    /// Transforms constraints from the conjugate subgroup to the original
    /// subgroup.
    fn constraints_to_subgroup(&self, mut constraint_set: ConstraintSet<P>) -> ConstraintSet<P> {
        for Constraint { from, to } in &mut constraint_set.constraints {
            *from = self.overgroup_action().act(self.rhs, *from);
            *to = self.overgroup_action().act(self.rhs, *to);
        }
        constraint_set
    }

    /// Transforms constraints from the original subgroup to the conjugate
    /// subgroup.
    fn constraints_from_subgroup(&self, mut constraint_set: ConstraintSet<P>) -> ConstraintSet<P> {
        for Constraint { from, to } in &mut constraint_set.constraints {
            *from = self.overgroup_action().act(self.lhs, *from);
            *to = self.overgroup_action().act(self.lhs, *to);
        }
        constraint_set
    }

    /// Unconjugates an element of the conjugate subgroup, which produces an
    /// element of the original subgroup.
    fn elem_to_subgroup(&self, elem: GroupElementId) -> GroupElementId {
        self.overgroup().conjugate(self.rhs, elem)
    }

    /// Conjugates an element of the original subgroup, which produces an
    /// element of the conjugate subgroup.
    fn elem_from_subgroup(&self, elem: GroupElementId) -> GroupElementId {
        self.overgroup().conjugate(self.lhs, elem)
    }

    /// Returns the coset within the conjugate subgroup satisfying a set of
    /// constraints, or `None` if there is no such coset.
    pub fn solve(&mut self, constraint_set: ConstraintSet<P>) -> Option<ConjugateCoset> {
        let ConjugateCoset {
            subgroup,
            mut lhs,
            mut rhs,
        } = self
            .subgroup_solver
            .solve(&self.constraints_to_subgroup(constraint_set))?;

        lhs = self.overgroup().compose(self.lhs, lhs);
        rhs = self.overgroup().compose(rhs, self.rhs);

        Some(ConjugateCoset { subgroup, lhs, rhs })
    }

    /// Selects an random element deterministically and uniformly from the
    /// conjugate subgroup that satisfies a set of constraints and returns a set
    /// of constraints that precisely specifies it.
    ///
    /// `random_index` must return a deterministic random integer in the range
    /// `0..n`. It will never be called with `n == 0`.
    ///
    /// Note that the deterministic random output is *not* necessarily the same
    /// as would be returned by [`crate::ConstraintSolver::select()`] in the
    /// larger group with an equivalent set of constraints.
    pub fn select(
        &mut self,
        constraint_set: ConstraintSet<P>,
        random_index: impl FnMut(usize) -> usize,
    ) -> Option<(ConstraintSet<P>, GroupElementId)> {
        let (constraint_set, element) = self
            .subgroup_solver
            .select(&self.constraints_to_subgroup(constraint_set), random_index)?;
        Some((
            self.constraints_from_subgroup(constraint_set),
            self.elem_from_subgroup(element),
        ))
    }

    /// Returns a canonical set of constraints that uniquely specifies an
    /// element within the conjugate subgroup, assuming the base constraints
    /// have already been applied.
    ///
    /// Returns `None` if `base_constraints` is unsatisfiable or if `element` is
    /// not in the subgroup.
    ///
    /// Panics in debug mode if `element` does not satisfy `base_constraints`.
    pub fn constraints_for_element(
        &mut self,
        base_constraints: ConstraintSet<P>,
        element: GroupElementId,
    ) -> Option<ConstraintSet<P>> {
        let ret = self.subgroup_solver.constraints_for_element(
            &self.constraints_to_subgroup(base_constraints),
            self.elem_to_subgroup(element),
        )?;
        Some(self.constraints_from_subgroup(ret))
    }
}
