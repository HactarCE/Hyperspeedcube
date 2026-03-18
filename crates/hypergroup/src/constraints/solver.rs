use std::collections::HashMap;
use std::sync::Arc;

use hypuz_util::ti::TiVec;
use itertools::Itertools;
use smallvec::{SmallVec, smallvec};

use super::SubgroupOrbits;
use crate::{
    AbstractGroupActionLut, AbstractGroupLut, AbstractSubgroup, ConjugateCoset, Constraint,
    ConstraintSet, GroupAction, GroupElementId, PerFactorGroup, ProductSubgroup, RefPoint,
};

hypuz_util::typed_index_struct! {
    /// ID of a subgroup within a [`StabilizersCache`].
    struct SubgroupId(usize);
}

type PerSubgroup<T> = TiVec<SubgroupId, T>;

impl SubgroupId {
    /// Subgroup that is includes all of the original group.
    const ORIGINAL_GROUP: Self = Self(0);
}

/// Same as [`ConjugateCoset`], but the subgroup is represented using
/// [`SubgroupId`].
pub(crate) struct FactorGroupConjugateCoset {
    /// Element to multiply on the left of the subgroup.
    pub lhs: GroupElementId,
    /// Subgroup.
    pub subgroup: Arc<AbstractSubgroup>,
    /// Element to multiple on the right of the subgroup.
    pub rhs: GroupElementId,
}

/// Solver for determining group elements/cosets that satisfy
/// [`ConstraintSet`]s.
///
/// Intermediate results are cached, so solvers should be reused whenever
/// possible.
pub struct ConstraintSolver {
    action: Arc<GroupAction>,
    solvers: PerFactorGroup<FactorGroupConstraintSolver>,
}

impl ConstraintSolver {
    pub fn new(action: Arc<GroupAction>) -> Self {
        let solvers = action
            .factors()
            .map_ref(|_, factor| FactorGroupConstraintSolver::new(Arc::clone(factor)));

        Self { action, solvers }
    }

    fn split_constraint_set(
        &self,
        constraint_set: &ConstraintSet,
    ) -> Option<PerFactorGroup<ConstraintSet>> {
        let mut constraint_set_for_each_factor = self.solvers.map_ref(|_, _| ConstraintSet::EMPTY);

        for Constraint { old, new } in constraint_set {
            let (factor, old) = self.action.ref_point_to_factor(old);
            let new = self.action.try_ref_point_to_factor(factor, new)?;
            constraint_set_for_each_factor[factor]
                .constraints
                .push(Constraint { old, new });
        }

        Some(constraint_set_for_each_factor)
    }

    fn merge_constraint_sets(
        &self,
        constraint_sets: PerFactorGroup<ConstraintSet>,
    ) -> ConstraintSet {
        constraint_sets
            .into_iter()
            .flat_map(|(factor, constraint_set)| {
                constraint_set
                    .into_iter()
                    .map(move |Constraint { old, new }| Constraint {
                        old: self.action.ref_point_from_factor(factor, old),
                        new: self.action.ref_point_from_factor(factor, new),
                    })
            })
            .collect()
    }

    pub fn solve(&mut self, constraint_set: &ConstraintSet) -> Option<ConjugateCoset> {
        let constraint_set_for_each_factor = self.split_constraint_set(constraint_set)?;

        let conjugate_cosets = self
            .solvers
            .iter_mut()
            .map(|(factor, solver)| solver.solve(&constraint_set_for_each_factor[factor]))
            .collect::<Option<PerFactorGroup<_>>>()?;

        let lhs = self
            .action
            .group()
            .element_from_factors(conjugate_cosets.iter_values().map(|cc| cc.lhs));
        let subgroup = Arc::new(ProductSubgroup::from_factors(
            self.action.group().clone(),
            conjugate_cosets
                .iter_values()
                .map(|cc| Arc::clone(&cc.subgroup)),
        ));
        let rhs = self
            .action
            .group()
            .element_from_factors(conjugate_cosets.iter_values().map(|cc| cc.rhs));

        Some(ConjugateCoset { lhs, subgroup, rhs })
    }

    pub fn select(
        &mut self,
        constraint_set: &ConstraintSet,
        mut select: impl FnMut(Vec<RefPoint>) -> Option<RefPoint>,
    ) -> Option<(ConstraintSet, GroupElementId)> {
        let constraint_set_for_each_factor = self.split_constraint_set(&constraint_set)?;

        let outputs = std::iter::zip(&mut self.solvers, constraint_set_for_each_factor)
            .map(|((_, solver), (_, constraint_set_for_factor))| {
                solver.select(constraint_set_for_factor, &mut select)
            })
            .collect::<Option<PerFactorGroup<_>>>()?;

        let combined_element = self.action.group().element_from_factors(
            outputs
                .iter_values()
                .map(|(_constraint_set, element_id)| *element_id),
        );
        let combined_constraint_set = self
            .merge_constraint_sets(outputs.map(|_, (constraint_set, _element_id)| constraint_set));

        // The combined set should include all the original constraints.
        #[cfg(debug_assertions)]
        for constraint in constraint_set {
            debug_assert!(combined_constraint_set.constraints.contains(&constraint));
        }

        // The combined constraint set should be satisfied by the element.
        #[cfg(debug_assertions)]
        for Constraint { old, new } in &combined_constraint_set {
            debug_assert_eq!(new, self.action.act(combined_element, old))
        }

        Some((combined_constraint_set, combined_element))
    }

    #[cfg(test)]
    pub(crate) fn total_subgroup_orbit_count(&self) -> usize {
        self.solvers
            .iter_values()
            .map(|solver| solver.subgroup_orbits.len())
            .sum()
    }
}

/// Solver for determining group elements/cosets that satisfy
/// [`ConstraintSet`]s.
pub(crate) struct FactorGroupConstraintSolver {
    action: Arc<AbstractGroupActionLut>,

    subgroup_orbits: PerSubgroup<SubgroupOrbits>,
    subgroups_by_fixed_points: HashMap<Box<[RefPoint]>, SubgroupId>,
    subgroups_by_generating_set: HashMap<Box<[GroupElementId]>, SubgroupId>,
}

impl FactorGroupConstraintSolver {
    /// Constructs a new constraint solver for a group action.
    pub fn new(action: Arc<AbstractGroupActionLut>) -> Self {
        let stabilizer_of_nothing = SubgroupOrbits::new_total(&action);
        let subgroups = PerSubgroup::from_iter([stabilizer_of_nothing]);
        let subgroups_by_fixed_points =
            HashMap::from_iter([(Box::from([]), SubgroupId::ORIGINAL_GROUP)]);
        let subgroups_by_generating_set = HashMap::from_iter([(
            Box::from(&**action.group().generators()),
            SubgroupId::ORIGINAL_GROUP,
        )]);

        Self {
            action,

            subgroup_orbits: subgroups,
            subgroups_by_fixed_points,
            subgroups_by_generating_set,
        }
    }

    /// Returns the abstract group.
    fn group(&self) -> &AbstractGroupLut {
        self.action.group()
    }

    /// Returns the pointwise stabilizer of the given points, using cached
    /// results when possible.
    ///
    /// See [`AbstractGroupActionLut::pointwise_stabilizer_generating_set()`].
    fn cached_pointwise_stabilizer(&mut self, fixed_points: &[RefPoint]) -> SubgroupId {
        // Fastest: cached by fixed points
        get_or_insert_entry_with(&mut self.subgroups_by_fixed_points, fixed_points, || {
            // Next fastest: cached by subgroup generators
            let subgroup = Arc::new(self.action.pointwise_stabilizer(fixed_points));
            let gen_set = subgroup.generators();
            get_or_insert_entry_with(&mut self.subgroups_by_generating_set, &gen_set, || {
                // Slowest: not cached
                let subgroup = SubgroupOrbits::new(&self.action, Arc::clone(&subgroup));
                self.subgroup_orbits
                    .push(subgroup)
                    .expect("subgroup overflow")
            })
        })
    }

    /// Solves a set of constraints and returns the coset satisfying it, or
    /// `None` if the constraints are unsatisfiable.
    fn solve_coset_impl(
        &mut self,
        constraint_set: &ConstraintSet,
    ) -> Option<ConstrainedConjugateCoset> {
        constraint_set
            .iter()
            .try_fold(ConstrainedConjugateCoset::new(), |coset, constraint| {
                self.constrain_coset(coset, constraint)
            })
    }

    /// Constrains the coset so that it takes `old` to `new`.
    ///
    /// Returns `None` if there is no such coset.
    fn constrain_coset(
        &mut self,
        mut coset: ConstrainedConjugateCoset,
        constraint: Constraint,
    ) -> Option<ConstrainedConjugateCoset> {
        let a = self.action.act(coset.rhs, constraint.old);
        let b = self.action.act(coset.lhs_inv, constraint.new);
        let orbits = &self.subgroup_orbits[coset.subgroup];
        if orbits.orbit_representatives[a] != orbits.orbit_representatives[b] {
            return None; // `a` and `b` are in different orbits
        }
        coset.fixed_points.push(orbits.orbit_representatives[a]);
        coset.rhs = self.group().compose(orbits.deorbiters[a], coset.rhs);
        coset.lhs_inv = self.group().compose(orbits.deorbiters[b], coset.lhs_inv);
        coset.subgroup = self.cached_pointwise_stabilizer(&coset.fixed_points);
        Some(coset)
    }

    /// When debug assertions are enabled, asserts that the conjugate coset
    /// satisfies the given constraints.
    ///
    /// When debug assertions are disabled, this does nothing.
    fn debug_assert_constraints(
        &self,
        coset: &ConstrainedConjugateCoset,
        expected_constraint_set: &ConstraintSet,
    ) {
        #[cfg(debug_assertions)]
        {
            let arbitrary_elem = self
                .group()
                .compose(self.group().inverse(coset.lhs_inv), coset.rhs);
            for pair in expected_constraint_set {
                debug_assert_eq!(self.action.act(arbitrary_elem, pair.old), pair.new);
            }
        }
    }

    /// Solves a set of constraints and returns the coset satisfying it, or
    /// `None` if the constraints are unsatisfiable.
    pub fn solve(&mut self, constraint_set: &ConstraintSet) -> Option<FactorGroupConjugateCoset> {
        let coset = self.solve_coset_impl(constraint_set)?;
        self.debug_assert_constraints(&coset, constraint_set);
        Some(FactorGroupConjugateCoset {
            lhs: self.group().inverse(coset.lhs_inv),
            subgroup: Arc::clone(&self.subgroup_orbits[coset.subgroup].subgroup),
            rhs: coset.rhs,
        })
    }

    /// Selects a random group element deterministically given a deterministic
    /// function for selecting a reference point from an **sorted** list.
    pub fn select(
        &mut self,
        mut constraint_set: ConstraintSet,
        mut select: impl FnMut(Vec<RefPoint>) -> Option<RefPoint>,
    ) -> Option<(ConstraintSet, GroupElementId)> {
        let mut coset = self.solve_coset_impl(&constraint_set)?;

        while !self.subgroup_orbits[coset.subgroup].subgroup.is_trivial() {
            let rhs_inv = self.group().inverse(coset.rhs);
            let lhs = self.group().inverse(coset.lhs_inv);
            let canonical_largest_orbit =
                &self.subgroup_orbits[coset.subgroup].canonical_largest_orbit;
            assert!(canonical_largest_orbit.len() > 1);
            let source_candidates = canonical_largest_orbit
                .iter()
                .map(|&p| self.action.act(rhs_inv, p))
                .collect_vec();
            let destination_candidates = canonical_largest_orbit
                .iter()
                .map(|&p| self.action.act(lhs, p))
                .collect_vec();
            let source = select(source_candidates)?;
            let destination = select(destination_candidates)?;
            let new_constraint = Constraint {
                old: source,
                new: destination,
            };
            constraint_set.constraints.push(new_constraint);
            coset = self.constrain_coset(coset, new_constraint)?;
        }

        self.debug_assert_constraints(&coset, &constraint_set);

        let lhs = self.group().inverse(coset.lhs_inv);
        let single_element_in_coset = self.group().compose(lhs, coset.rhs);

        Some((constraint_set, single_element_in_coset))
    }
}

/// [`ConjugateCoset`] with inverted left-hand side: `lhs_inv^-1 * subgroup *
/// rhs`.
///
/// Constraints are added to the coset until the subgroup contains only
/// the identity, at which point `lhs_inv^-1 * rhs` satisfies all the
/// constraints.
struct ConstrainedConjugateCoset {
    fixed_points: SmallVec<[RefPoint; 8]>,
    lhs_inv: GroupElementId,
    rhs: GroupElementId,
    /// Subgroup that pointwise-stabilizes `fixed_points`.
    subgroup: SubgroupId,
}

impl ConstrainedConjugateCoset {
    fn new() -> Self {
        Self {
            fixed_points: smallvec![],
            lhs_inv: GroupElementId::IDENTITY,
            rhs: GroupElementId::IDENTITY,
            subgroup: SubgroupId::ORIGINAL_GROUP, // stabilizer of empty `fixed_points`
        }
    }
}

fn get_or_insert_entry_with<'a, K: 'a + Clone + std::hash::Hash + Eq, V: Copy>(
    hashmap: &mut HashMap<Box<[K]>, V>,
    key: &[K],
    default: impl FnOnce() -> V,
) -> V {
    hashmap.get(key).copied().unwrap_or_else(|| {
        *hashmap
            .entry(key.to_vec().into_boxed_slice())
            .or_insert_with(default)
    })
}
