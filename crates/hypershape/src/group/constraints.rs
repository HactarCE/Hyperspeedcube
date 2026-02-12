use std::collections::HashMap;
use std::sync::Arc;

use hypuz_util::ti::TiVec;
use smallvec::SmallVec;

use super::{Coset, Group, GroupAction, GroupElementId, RefPoint, Subgroup, SubgroupOrbits};

/// Constraint on a group element based on how it acts on reference points.
///
/// An element `g` satisfies this constraint if `g * old = new`.
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, Hash)]
pub struct Constraint {
    /// Original point.
    pub old: RefPoint,
    /// Transformed point.
    pub new: RefPoint,
}

impl From<[RefPoint; 2]> for Constraint {
    fn from([old, new]: [RefPoint; 2]) -> Self {
        Self { old, new }
    }
}

/// Set of constraints on a group element based on how it acts on reference
/// points.
///
/// This is used to specify a group element in a way that depends only on the
/// reference points (which can be assigned standard names), irrespective of the
/// IDs assigned to specific group elements.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ConstraintSet {
    /// List of constraints in arbitrary order.
    pub constraints: SmallVec<[Constraint; 4]>,
}

impl From<&[[RefPoint; 2]]> for ConstraintSet {
    fn from(pairs: &[[RefPoint; 2]]) -> Self {
        Self {
            constraints: pairs
                .iter()
                .map(|&[old, new]| Constraint { old, new })
                .collect(),
        }
    }
}

impl<const N: usize> From<[[RefPoint; 2]; N]> for ConstraintSet {
    fn from(value: [[RefPoint; 2]; N]) -> Self {
        Self::from(value.as_slice())
    }
}

hypuz_util::typed_index_struct! {
    /// ID of a subgroup within a [`StabilizersCache`].
    struct SubgroupId(usize);
}

type PerSubgroup<T> = TiVec<SubgroupId, T>;

impl SubgroupId {
    /// Subgroup that is actually the original group.
    const ORIGINAL_GROUP: Self = Self(0);
}

/// Solver for determining group elements/cosets from [`ConstraintSet`]s.
///
/// This is expensive to construct and to use with new constraint subgroups, so
/// it should be reused whenever possible.
pub struct ConstraintSolver {
    action: Arc<GroupAction>,

    subgroups: PerSubgroup<Subgroup>,
    subgroup_orbits: PerSubgroup<SubgroupOrbits>,
    subgroups_by_fixed_points: HashMap<Box<[RefPoint]>, SubgroupId>,
    subgroups_by_generating_set: HashMap<Box<[GroupElementId]>, SubgroupId>,
}

impl ConstraintSolver {
    /// Constructs a new constraint solver for a group action.
    pub fn new(action: Arc<GroupAction>) -> Self {
        let stabilizer_of_nothing = action.pointwise_stabilizer(&[]);
        let subgroup_orbits = PerSubgroup::from_iter([action.orbits(&stabilizer_of_nothing)]);
        let subgroups_by_fixed_points =
            HashMap::from_iter([(Box::from([]), SubgroupId::ORIGINAL_GROUP)]);
        let subgroups_by_generating_set = HashMap::from_iter([(
            Box::from(stabilizer_of_nothing.generating_set()),
            SubgroupId::ORIGINAL_GROUP,
        )]);

        Self {
            action,

            subgroups: PerSubgroup::from_iter([stabilizer_of_nothing]),
            subgroup_orbits,
            subgroups_by_fixed_points,
            subgroups_by_generating_set,
        }
    }

    /// Returns the pointwise stabilizer of the given points, caching results
    /// when possible.
    ///
    /// See [`GroupAction::pointwise_stabilizer()`].
    fn pointwise_stabilizer(&mut self, fixed_points: &[RefPoint]) -> SubgroupId {
        // Fastest: cached by fixed points
        if let Some(&id) = self.subgroups_by_fixed_points.get(fixed_points) {
            return id;
        }

        // Next fastest: cached by subgroup generators
        let subgroup = self.action.pointwise_stabilizer(fixed_points);
        if let Some(&id) = self
            .subgroups_by_generating_set
            .get(subgroup.generating_set())
        {
            self.subgroups_by_fixed_points
                .insert(Box::from(fixed_points), id);
            return id;
        }

        // Slowest: not cached
        let generating_set = Box::from(subgroup.generating_set());
        let subgroup_orbits = self.action.orbits(&subgroup);
        let id = self
            .subgroups
            .push(subgroup)
            .expect("usize shouldn't overflow");
        self.subgroup_orbits
            .push(subgroup_orbits)
            .expect("usize shouldn't overflow");
        self.subgroups_by_generating_set.insert(generating_set, id);
        self.subgroups_by_fixed_points
            .insert(fixed_points.to_vec().into_boxed_slice(), id);

        id
    }

    /// Solves a set of constraints and returns the coset satisfying it, or
    /// `None` if the constraints are unsatisfiable.
    pub fn solve(&mut self, constraint_set: &ConstraintSet) -> Option<Coset<'_>> {
        let mut fixed_points = Vec::with_capacity(constraint_set.constraints.len());
        let mut subgroup = SubgroupId::ORIGINAL_GROUP; // stabilizer of `fixed_points`
        let mut lhs_inv = GroupElementId::IDENTITY;
        let mut rhs = GroupElementId::IDENTITY;
        for &pair in &constraint_set.constraints {
            let a = self.action.act(rhs, pair.old);
            let b = self.action.act(lhs_inv, pair.new);
            let a_deorbiter = self.subgroup_orbits[subgroup].deorbiters[a];
            let b_deorbiter = self.subgroup_orbits[subgroup].deorbiters[b];
            if a_deorbiter.orbit_representative != b_deorbiter.orbit_representative {
                return None;
            }
            fixed_points.push(a_deorbiter.orbit_representative);
            subgroup = self.pointwise_stabilizer(&fixed_points);
            rhs = self.action.compose(a_deorbiter.deorbiter, rhs);
            lhs_inv = self.action.compose(b_deorbiter.deorbiter, lhs_inv);
        }
        let lhs = self.action.inverse(lhs_inv);

        let offset = self.action.compose(lhs, rhs);

        for &pair in &constraint_set.constraints {
            debug_assert_eq!(self.action.act(offset, pair.old), pair.new);
        }

        Some(Coset {
            subgroup: &self.subgroups[subgroup],
            offset,
        })
    }
}

#[cfg(test)]
mod tests {
    use hypermath::{Point, point};

    use super::*;

    #[test]
    pub fn test_group_element_constraint_solver() -> eyre::Result<()> {
        #![allow(non_snake_case)]

        let group = crate::FiniteCoxeterGroup::H3.coxeter_group(None)?.group()?;
        let chiral_group = crate::FiniteCoxeterGroup::H3
            .coxeter_group(None)?
            .chiral_group()?;

        let g0 = &group[crate::GeneratorId(0)];
        let g1 = &group[crate::GeneratorId(1)];
        let g2 = &group[crate::GeneratorId(2)];
        let mut ref_points = TiVec::<RefPoint, Point>::new();

        let F = ref_points.push(point![0.0, 0.0, 1.0])?;
        let U = ref_points.push(g2.transform(&ref_points[F]))?;
        let R = ref_points.push(g1.transform(&ref_points[U]))?;
        let L = ref_points.push(g0.transform(&ref_points[R]))?;
        let DR = ref_points.push(g1.transform(&ref_points[L]))?;
        let DL = ref_points.push(g0.transform(&ref_points[DR]))?;
        let BR = ref_points.push(g2.transform(&ref_points[DR]))?;
        let BL = ref_points.push(g2.transform(&ref_points[DL]))?;
        let PR = ref_points.push(g1.transform(&ref_points[BL]))?;
        let PL = ref_points.push(g0.transform(&ref_points[PR]))?;
        let PD = ref_points.push(g1.transform(&ref_points[PL]))?;
        #[allow(unused)]
        let PB = ref_points.push(g2.transform(&ref_points[PD]))?;

        let mut solver =
            ConstraintSolver::new(Arc::new(group.action_on_points(&ref_points).unwrap()));
        let mut chiral_solver = ConstraintSolver::new(Arc::new(
            chiral_group.action_on_points(&ref_points).unwrap(),
        ));

        for (constraint_set, expected_order, expected_chiral_order) in [
            ([].as_slice(), 120, 60),
            (&[[F, F]], 10, 5),
            (&[[R, L]], 10, 5),
            (&[[BL, R], [DR, PL]], 10, 5),
            (&[[DR, U], [L, DL]], 2, 1),
            (&[[DR, U], [L, DL], [R, BL]], 1, 1),
            (&[[DR, R], [L, L], [R, BR]], 1, 1),
            (&[[DR, U], [L, DL], [PR, BR]], 2, 1),
        ] {
            println!("Computing {constraint_set:?} ...");

            let t = std::time::Instant::now();

            let coset = solver.solve(&constraint_set.into()).unwrap();
            assert_eq!(coset.subgroup.element_count(), expected_order);

            let chiral_coset = chiral_solver.solve(&constraint_set.into()).unwrap();
            assert_eq!(chiral_coset.subgroup.element_count(), expected_chiral_order);

            println!(
                "Computed {} constraints in {:?}",
                constraint_set.len(),
                t.elapsed(),
            );
        }

        assert!(solver.solve(&[[DR, U], [L, DL], [R, L]].into()).is_none());
        assert!(
            chiral_solver
                .solve(&[[DR, U], [L, DL], [R, L]].into())
                .is_none()
        );

        assert_eq!(
            solver
                .solve(&[[U, U], [L, R], [F, F]].into())
                .unwrap()
                .subgroup
                .element_count(),
            1,
        );
        assert!(
            chiral_solver
                .solve(&[[U, U], [L, R], [F, F]].into())
                .is_none()
        );

        assert_eq!(solver.subgroups.len(), 5); // 120, 10, 5, 2, 1
        assert_eq!(chiral_solver.subgroups.len(), 3); // 60, 5, 1

        Ok(())
    }
}
