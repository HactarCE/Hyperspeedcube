use std::collections::HashMap;
use std::sync::Arc;

use hypuz_util::ti::TiVec;
use itertools::Itertools;
use smallvec::{SmallVec, smallvec};

use super::{
    ConjugateCoset, Group, GroupAction, GroupElementId, RefPoint, Subgroup, SubgroupOrbits,
};

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
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
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

impl FromIterator<Constraint> for ConstraintSet {
    fn from_iter<T: IntoIterator<Item = Constraint>>(iter: T) -> Self {
        Self {
            constraints: iter.into_iter().collect(),
        }
    }
}

impl ConstraintSet {
    /// Empty constraint set.
    pub const EMPTY: Self = Self {
        constraints: SmallVec::new_const(),
    };
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
    pub fn solve(&mut self, constraint_set: &ConstraintSet) -> Option<ConjugateCoset<&Subgroup>> {
        let coset = ConstrainedConjugateCoset::from_constraints(self, constraint_set)?;
        coset.debug_assert_constraints(constraint_set);
        Some(coset.into())
    }

    /// Selects a random group element deterministically given a deterministic
    /// function for selecting a reference point from an **sorted** list.
    pub fn select(
        &mut self,
        mut constraint_set: ConstraintSet,
        mut select: impl FnMut(Vec<RefPoint>) -> Option<RefPoint>,
    ) -> Option<(ConstraintSet, GroupElementId)> {
        let mut coset = ConstrainedConjugateCoset::from_constraints(self, &constraint_set)?;

        while coset.solver.subgroups[coset.subgroup].element_count() > 1 {
            let rhs_inv = coset.solver.action.inverse(coset.rhs);
            let lhs = coset.solver.action.inverse(coset.lhs_inv);
            let canonical_largest_orbit =
                &coset.solver.subgroup_orbits[coset.subgroup].canonical_largest_orbit;
            assert!(canonical_largest_orbit.len() > 1);
            let source_candidates = canonical_largest_orbit
                .iter()
                .map(|&p| coset.solver.action.act(rhs_inv, p))
                .collect_vec();
            let destination_candidates = canonical_largest_orbit
                .iter()
                .map(|&p| coset.solver.action.act(lhs, p))
                .collect_vec();
            let source = select(source_candidates)?;
            let destination = select(destination_candidates)?;
            let new_constraint = Constraint {
                old: source,
                new: destination,
            };
            constraint_set.constraints.push(new_constraint);
            coset = coset.constrain(new_constraint)?;
        }

        coset.debug_assert_constraints(&constraint_set);

        Some((constraint_set, coset.to_element()?))
    }
}

/// [`ConjugateCoset`] with inverted left-hand side: `lhs_inv^-1 * subgroup *
/// rhs`.
///
/// Constraints are added to the coset until the subgroup contains only
/// the identity, at which point `lhs_inv^-1 * rhs` satisfies all the
/// constraints.
struct ConstrainedConjugateCoset<'a> {
    solver: &'a mut ConstraintSolver,
    fixed_points: SmallVec<[RefPoint; 8]>,
    /// Subgroup that pointwise-stabilizes `fixed_points`.
    subgroup: SubgroupId,
    lhs_inv: GroupElementId,
    rhs: GroupElementId,
}

impl<'a> ConstrainedConjugateCoset<'a> {
    fn new(solver: &'a mut ConstraintSolver) -> Self {
        Self {
            solver,
            fixed_points: smallvec![],
            subgroup: SubgroupId::ORIGINAL_GROUP, // stabilizer of empty `fixed_points`
            lhs_inv: GroupElementId::IDENTITY,
            rhs: GroupElementId::IDENTITY,
        }
    }

    fn from_constraints(
        solver: &'a mut ConstraintSolver,
        constraint_set: &ConstraintSet,
    ) -> Option<Self> {
        let mut ret = Self::new(solver);
        for &constraint in &constraint_set.constraints {
            ret = ret.constrain(constraint)?;
        }
        Some(ret)
    }

    /// Constrains the coset so that it takes `old` to `new`.
    ///
    /// Returns `None` if there is no such coset.
    fn constrain(mut self, constraint: Constraint) -> Option<ConstrainedConjugateCoset<'a>> {
        let solver = &mut *self.solver;
        let a = solver.action.act(self.rhs, constraint.old);
        let b = solver.action.act(self.lhs_inv, constraint.new);
        let a_deorbiter = solver.subgroup_orbits[self.subgroup].deorbiters[a];
        let b_deorbiter = solver.subgroup_orbits[self.subgroup].deorbiters[b];
        if a_deorbiter.orbit_representative != b_deorbiter.orbit_representative {
            return None;
        }
        self.fixed_points.push(a_deorbiter.orbit_representative);
        self.subgroup = solver.pointwise_stabilizer(&self.fixed_points);
        self.rhs = solver.action.compose(a_deorbiter.deorbiter, self.rhs);
        self.lhs_inv = solver.action.compose(b_deorbiter.deorbiter, self.lhs_inv);
        Some(self)
    }

    /// When debug assertions are enabled, asserts that the conjugate coset
    /// satisfies the given constraints.
    ///
    /// When debug assertions are disabled, this does nothing.
    fn debug_assert_constraints(&self, constraint_set: &ConstraintSet) {
        #[cfg(debug_assertions)]
        {
            let elem = self.arbitrary_element();
            for &pair in &constraint_set.constraints {
                debug_assert_eq!(self.solver.action.act(elem, pair.old), pair.new);
            }
        }
    }

    /// Returns an arbitrary group element in the conjugate coset.
    fn arbitrary_element(&self) -> GroupElementId {
        self.solver
            .action
            .compose(self.solver.action.inverse(self.lhs_inv), self.rhs)
    }

    /// Returns the single group element in the conjugate coset, if it contains
    /// only one element.
    fn to_element(&self) -> Option<GroupElementId> {
        self.solver.subgroups[self.subgroup].is_trivial().then(|| {
            let lhs = self.solver.action.inverse(self.lhs_inv);
            self.solver.action.compose(lhs, self.rhs)
        })
    }
}

impl<'a> From<ConstrainedConjugateCoset<'a>> for ConjugateCoset<&'a Subgroup> {
    fn from(value: ConstrainedConjugateCoset<'a>) -> Self {
        Self {
            lhs: value.solver.action.inverse(value.lhs_inv),
            subgroup: &value.solver.subgroups[value.subgroup],
            rhs: value.rhs,
        }
    }
}

#[cfg(test)]
mod tests {
    use hypermath::pga::Motor;
    use hypermath::{APPROX, ApproxHashMap, Point, Vector, point};
    use rand::seq::IndexedRandom;
    use rand::{Rng, RngExt, SeedableRng};

    use super::*;
    use crate::{GenSeq, GeneratorId, IsometryGroup, PerRefPoint, orbit_geometric};

    /// Tests the constraint solver on Coxeter group H3 (dodecahedral symmetry)
    #[test]
    fn test_group_element_constraint_solver_h3() -> eyre::Result<()> {
        #![allow(non_snake_case)]

        let group = crate::FiniteCoxeterGroup::H3.coxeter_group(None)?.group()?;
        let chiral_group = crate::FiniteCoxeterGroup::H3
            .coxeter_group(None)?
            .chiral_group()?;

        let g0 = &group[GeneratorId(0)];
        let g1 = &group[GeneratorId(1)];
        let g2 = &group[GeneratorId(2)];
        let mut ref_points = PerRefPoint::<Point>::new();

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
        #[expect(unused)]
        let PB = ref_points.push(g2.transform(&ref_points[PD]))?;

        let mut solver = ConstraintSolver::new(Arc::new(group.action_on_points(&ref_points)?));
        let mut chiral_solver = ConstraintSolver::new(Arc::new(
            chiral_group.action_on_points(&ref_points).unwrap(),
        ));

        for (constraint_set, expected_order, expected_chiral_order) in [
            ([].as_slice(), 120, 60),
            (&[[F, F]], 10, 5),
            (&[[R, L]], 10, 5),
            (&[[BL, R], [DR, PL]], 10, 5), // opposites
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

    /// Tests the constraint solver on Coxeter group A4 (4-simplex symmetry)
    #[test]
    fn test_group_element_constraint_solver_a4() -> eyre::Result<()> {
        #![allow(non_snake_case)]

        let group = crate::FiniteCoxeterGroup::A(4)
            .coxeter_group(None)?
            .group()?;
        let chiral_group = crate::FiniteCoxeterGroup::A(4)
            .coxeter_group(None)?
            .chiral_group()?;

        let gen0 = GeneratorId(0);
        let gen1 = GeneratorId(1);
        let gen2 = GeneratorId(2);
        let gen3 = GeneratorId(3);

        let mut ref_points = PerRefPoint::<Point>::new();
        let E = ref_points.push(point![0.0, 0.0, 0.0, 1.0])?;
        let D = ref_points.push(group[gen3].transform(&ref_points[E]))?;
        let C = ref_points.push(group[gen2].transform(&ref_points[D]))?;
        let B = ref_points.push(group[gen1].transform(&ref_points[C]))?;
        let A = ref_points.push(group[gen0].transform(&ref_points[B]))?;

        let mut solver = ConstraintSolver::new(Arc::new(group.action_on_points(&ref_points)?));
        let mut chiral_solver = ConstraintSolver::new(Arc::new(
            chiral_group.action_on_points(&ref_points).unwrap(),
        ));

        for (constraint_set, expected_order, expected_chiral_order) in [
            ([].as_slice(), 120, 60),
            (&[[A, C], [B, D], [D, E]], 2, 1),
            (&[[A, C], [B, D], [D, E], [C, B]], 1, 1),
            (&[[A, C], [B, D], [D, E], [C, B], [E, A]], 1, 1),
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

        assert_eq!(solver.subgroups.len(), 5); // 120, 24, 6, 2, 1
        assert_eq!(chiral_solver.subgroups.len(), 4); // 60, 12, 3, 1

        Ok(())
    }

    #[test]
    fn test_deterministic_random_group_element() -> eyre::Result<()> {
        #![allow(non_snake_case)]

        let coxeter_group = crate::FiniteCoxeterGroup::H4.coxeter_group(None)?;
        let initial_point = Point(Vector::unit(coxeter_group.min_ndim() - 1));
        let ref_points: PerRefPoint<Point> = orbit_geometric(
            &coxeter_group
                .generator_motors()
                .into_iter()
                .enumerate()
                .map(|(i, m)| (GenSeq::new([GeneratorId(i as u8)]), m))
                .collect_vec(),
            initial_point,
        )
        .into_iter()
        .map(|(_, _, p)| p)
        .collect();
        assert_eq!(ref_points.len(), 120);

        let original_group = coxeter_group.group()?;
        let group_1 = shuffle_group_generators(
            &original_group,
            &mut rand::rngs::StdRng::seed_from_u64(0xABCD),
        );
        let group_2 = shuffle_group_generators(
            &original_group,
            &mut rand::rngs::StdRng::seed_from_u64(0xDCBA),
        );

        let mut select_rng_1 = rand::rngs::StdRng::seed_from_u64(123456789);
        let mut select_rng_2 = rand::rngs::StdRng::seed_from_u64(123456789);

        let count = 1_000;

        let selected_1;
        {
            let action_1 = group_1.action_on_points(&ref_points).unwrap();
            let mut solver_1 = ConstraintSolver::new(Arc::new(action_1));

            selected_1 = (0..count)
                .map(|_| {
                    solver_1
                        .select(ConstraintSet::EMPTY, |mut points| {
                            points.sort(); // TODO: select_nth_unstable()
                            points.choose(&mut select_rng_1).copied()
                        })
                        .expect("no point satisfying constraints")
                })
                .map(|(constraint_set, _elem)| constraint_set)
                .collect_vec();
        }

        let selected_2;
        {
            let action_2 = group_2.action_on_points(&ref_points).unwrap();
            let mut solver_2 = ConstraintSolver::new(Arc::new(action_2));

            selected_2 = (0..count)
                .map(|_| {
                    solver_2
                        .select(ConstraintSet::EMPTY, |mut points| {
                            points.sort(); // TODO: select_nth_unstable()
                            points.choose(&mut select_rng_2).copied()
                        })
                        .expect("no point satisfying constraints")
                })
                .map(|(constraint_set, _elem)| constraint_set)
                .collect_vec();
        }

        assert_eq!(selected_1, selected_2);

        Ok(())
    }

    /// By running the product replacement algorithm on the generators for a
    /// group before generating the group, we can get much shorter words for the
    /// elements on average. This makes multiplying group elements and
    /// transforming points much faster.
    ///
    /// We can use the same number of generators, or add more generators. More
    /// generators yields shorter words, but with diminishing returns. More
    /// generators also requires more iterations of the product replacement
    /// algorithm.
    ///
    /// Empirically, most 3D and 4D groups (I tested H3 and H4) only need ~10
    /// iterations to converge when not adding more generators. I100 takes
    /// *many* more iterations (somewhere between 50 and 100), especially when
    /// adding more generators. It may be worth running a few hundred iterations
    /// on all groups to play it safe, especially considering product
    /// replacement is so cheap to compute.
    #[test]
    fn product_replacement_word_len() -> eyre::Result<()> {
        let group = crate::FiniteCoxeterGroup::I(100)
            .coxeter_group(None)?
            .group()?;

        let generators = group
            .generators()
            .iter_values()
            .map(|&e| group[e].clone())
            .collect_vec();

        let mut permute_rng_1 = rand::rngs::StdRng::seed_from_u64(987654321);

        let mut generators = generators.clone();
        generators.resize(generators.len() * 3, Motor::ident(0));
        for i in 0..100 {
            let mut unique_generators = ApproxHashMap::<Motor, ()>::new(APPROX);
            for g in &generators {
                unique_generators.insert(g.clone(), ());
            }
            unique_generators.remove(Motor::ident(0));
            let group = IsometryGroup::from_generators(
                &unique_generators
                    .iter()
                    .map(|(k, _)| k.clone())
                    .filter(|g| !g.is_ident())
                    .collect_vec(),
            )
            .unwrap();
            let avg_word_len = group
                .elements()
                .map(|e| group.factorization(e).len())
                .sum::<usize>() as f32
                / group.element_count() as f32;
            println!("{i} {avg_word_len}");

            let mut indices = (0..generators.len()).collect_vec();
            let i = indices.swap_remove(permute_rng_1.random_range(0..generators.len()));
            let j = *indices.choose(&mut permute_rng_1).unwrap();
            generators[i] = generators[i].clone() * &generators[j];
        }
        Ok(())
    }

    fn shuffle_group_generators(group: &IsometryGroup, rng: &mut impl Rng) -> IsometryGroup {
        let mut generators = group
            .generators()
            .iter_values()
            .map(|&e| group[e].clone())
            .collect_vec();
        for _ in 0..20 {
            let i = rng.random_range(0..generators.len());
            let mut j = rng.random_range(0..generators.len() - 1);
            if j >= i {
                j += 1;
            }
            generators[i] = &generators[i] * &generators[j];
        }
        IsometryGroup::from_generators(&generators).unwrap()
    }
}
