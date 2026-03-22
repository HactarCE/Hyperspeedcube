use std::sync::Arc;
use std::{collections::HashMap, fmt};

use hypuz_util::ti::{TiVec, TypedIndex};
use itertools::Itertools;
use smallvec::{SmallVec, smallvec};

use super::SubgroupOrbits;
use crate::{
    AbstractGroupActionLut, AbstractGroupLut, AbstractSubgroup, Constraint, ConstraintSet, Coset,
    GroupAction, GroupElementId, PerFactorGroup,
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
    /// Element to multiply on the right of the subgroup.
    pub rhs: GroupElementId,
}

/// Solver for determining group elements/cosets that satisfy
/// [`ConstraintSet`]s.
///
/// Intermediate results are cached, so solvers should be reused whenever
/// possible.
#[derive(Debug)]
pub struct ConstraintSolver<P> {
    action: GroupAction<P>,
    factor_solvers: PerFactorGroup<FactorGroupConstraintSolver<P>>,
}

impl<P: TypedIndex> ConstraintSolver<P> {
    /// Constructs a new constraint solver.
    pub fn new(action: GroupAction<P>) -> Self {
        let solvers = action
            .factors()
            .map_ref(|_, factor| FactorGroupConstraintSolver::new(Arc::clone(factor)));

        Self {
            action,
            factor_solvers: solvers,
        }
    }

    /// Splits a constraint set into a constraint set for each factor. Returns
    /// `None` if any constraint spans multiple factors, making it
    /// unsatisfiable.
    fn split_constraint_set(
        &self,
        constraint_set: &ConstraintSet<P>,
    ) -> Option<PerFactorGroup<ConstraintSet<P>>> {
        let mut constraint_set_for_each_factor =
            self.factor_solvers.map_ref(|_, _| ConstraintSet::EMPTY);

        for Constraint { from, to } in constraint_set {
            let (factor, from) = self.action.point_to_factor(from);
            let to = self.action.try_point_to_factor(factor, to)?;
            constraint_set_for_each_factor[factor]
                .constraints
                .push(Constraint { from, to });
        }

        Some(constraint_set_for_each_factor)
    }

    fn merge_constraint_sets(
        &self,
        constraint_sets: PerFactorGroup<ConstraintSet<P>>,
    ) -> ConstraintSet<P> {
        constraint_sets
            .into_iter()
            .flat_map(|(factor, constraint_set)| {
                constraint_set
                    .into_iter()
                    .map(move |Constraint { from, to }| Constraint {
                        from: self.action.point_from_factor(factor, from),
                        to: self.action.point_from_factor(factor, to),
                    })
            })
            .collect()
    }

    /// Returns the coset satisfying a set of constraints, or `None` if there is
    /// no such coset.
    pub fn solve(&mut self, constraint_set: &ConstraintSet<P>) -> Option<Coset> {
        let group = self.action.group();

        let constraint_set_for_each_factor = self.split_constraint_set(constraint_set)?;

        let conjugate_cosets = self
            .factor_solvers
            .iter_mut()
            .map(|(factor, solver)| solver.solve(&constraint_set_for_each_factor[factor]))
            .collect::<Option<PerFactorGroup<_>>>()?;

        let lhs = self
            .action
            .group()
            .element_from_factors(conjugate_cosets.iter_values().map(|cc| cc.lhs));

        let rhs = self
            .action
            .group()
            .element_from_factors(conjugate_cosets.iter_values().map(|cc| cc.rhs));

        #[cfg(debug_assertions)]
        for Constraint { from, to } in constraint_set {
            debug_assert_eq!(
                to,
                self.action.act(group.compose(lhs, rhs), from),
                "coset does not satisfy constraints",
            );
        }

        // This shouldn't overflow because it must be no larger than
        // `overgroup.element_count`.
        let subgroup_element_count = conjugate_cosets
            .iter_values()
            .map(|cc| cc.subgroup.element_count())
            .product();

        let subgroup_generators = conjugate_cosets.iter().flat_map(|(factor, cc)| {
            cc.subgroup
                .generators()
                .iter()
                .map(move |&g| group.element_from_factor(factor, g))
        });

        Some(Coset::from_conjugate_coset(
            group.clone(),
            subgroup_element_count,
            lhs,
            subgroup_generators,
            rhs,
        ))
    }

    /// Selects an random element deterministically and uniformly from the group
    /// that satisfies a set of constraints and returns a set of constraints
    /// that precisely specifies it.
    ///
    /// `random_index` must return a deterministic random integer in the range
    /// `0..n`. The range will never be empty.
    pub fn select(
        &mut self,
        constraint_set: &ConstraintSet<P>,
        mut random_index: impl FnMut(usize) -> usize,
    ) -> Option<(ConstraintSet<P>, GroupElementId)> {
        let constraint_set_for_each_factor = self.split_constraint_set(&constraint_set)?;

        let outputs = std::iter::zip(&mut self.factor_solvers, constraint_set_for_each_factor)
            .map(|((_, solver), (_, constraint_set_for_factor))| {
                solver.select(constraint_set_for_factor, &mut random_index)
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
        for Constraint { from, to } in &combined_constraint_set {
            debug_assert_eq!(to, self.action.act(combined_element, from))
        }

        Some((combined_constraint_set, combined_element))
    }

    /// Returns a canonical set of constraints that uniquely specifies an
    /// element, assuming the base constraints have already been applied.
    ///
    /// Returns `None` if `base_constraints` is unsatisfiable.
    ///
    /// Panics in debug mode if `element` does not satisfy `base_constraints`.
    pub fn constraints_for_element(
        &mut self,
        base_constraints: &ConstraintSet<P>,
        element: GroupElementId,
    ) -> Option<ConstraintSet<P>> {
        let constraint_set_for_each_factor = self.split_constraint_set(&base_constraints)?;
        let outputs = itertools::izip!(
            &mut self.factor_solvers,
            constraint_set_for_each_factor,
            self.action.group().element_to_factors(element)
        )
        .map(|((_, solver), (_, base_constraints_for_factor), element)| {
            solver.constraints_for_element(&base_constraints_for_factor, element)
        })
        .collect::<Option<PerFactorGroup<_>>>()?;
        let combined_constraint_set = self.merge_constraint_sets(outputs);

        debug_assert_eq!(
            element,
            self.solve(&ConstraintSet::from_iter(std::iter::chain(
                base_constraints,
                &combined_constraint_set
            )))
            .unwrap()
            .elements()
            .into_iter()
            .exactly_one()
            .unwrap(),
        );

        Some(combined_constraint_set)
    }

    #[cfg(test)]
    pub(crate) fn total_subgroup_orbit_count(&self) -> usize {
        self.factor_solvers
            .iter_values()
            .map(|solver| solver.subgroup_orbits.len())
            .sum()
    }
}

/// Solver for determining group elements/cosets that satisfy
/// [`ConstraintSet`]s.
pub(crate) struct FactorGroupConstraintSolver<P> {
    action: Arc<AbstractGroupActionLut<P>>,

    subgroup_orbits: PerSubgroup<SubgroupOrbits<P>>,
    subgroups_by_fixed_points: HashMap<Box<[P]>, SubgroupId>,
    subgroups_by_generating_set: HashMap<Box<[GroupElementId]>, SubgroupId>,
}

impl<P: fmt::Debug> fmt::Debug for FactorGroupConstraintSolver<P> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FactorGroupConstraintSolver")
            .field("subgroup_count", &self.subgroup_orbits.len())
            .finish_non_exhaustive()
    }
}

impl<P: TypedIndex> FactorGroupConstraintSolver<P> {
    /// Constructs a new constraint solver for a group action.
    pub fn new(action: Arc<AbstractGroupActionLut<P>>) -> Self {
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
    fn cached_pointwise_stabilizer(&mut self, fixed_points: &[P]) -> SubgroupId {
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
        constraint_set: &ConstraintSet<P>,
    ) -> Option<ConstrainedConjugateCoset<P>> {
        constraint_set
            .iter()
            .try_fold(ConstrainedConjugateCoset::new(), |coset, constraint| {
                self.constrain_coset(coset, constraint)
            })
    }

    /// Constrains the coset so that it takes `from` to `new`.
    ///
    /// Returns `None` if there is no such coset.
    fn constrain_coset(
        &mut self,
        mut coset: ConstrainedConjugateCoset<P>,
        constraint: Constraint<P>,
    ) -> Option<ConstrainedConjugateCoset<P>> {
        let a = self.action.act(coset.rhs, constraint.from);
        let b = self.action.act(coset.lhs_inv, constraint.to);
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
        coset: &ConstrainedConjugateCoset<P>,
        expected_constraint_set: &ConstraintSet<P>,
    ) {
        #[cfg(debug_assertions)]
        {
            let arbitrary_elem = self
                .group()
                .compose(self.group().inverse(coset.lhs_inv), coset.rhs);
            for pair in expected_constraint_set {
                debug_assert_eq!(self.action.act(arbitrary_elem, pair.from), pair.to);
            }
        }
    }

    /// Solves a set of constraints and returns the coset satisfying it, or
    /// `None` if the constraints are unsatisfiable.
    pub fn solve(
        &mut self,
        constraint_set: &ConstraintSet<P>,
    ) -> Option<FactorGroupConjugateCoset> {
        let coset = self.solve_coset_impl(constraint_set)?;
        self.debug_assert_constraints(&coset, constraint_set);
        Some(FactorGroupConjugateCoset {
            lhs: self.group().inverse(coset.lhs_inv),
            subgroup: Arc::clone(&self.subgroup_orbits[coset.subgroup].subgroup),
            rhs: coset.rhs,
        })
    }

    /// Selects a random element deterministically and uniformly from the group.
    ///
    /// `random_index` must return a deterministic random integer in the range
    /// `0..n`. The range will never be empty.
    pub fn select(
        &mut self,
        mut constraint_set: ConstraintSet<P>,
        mut random_index: impl FnMut(usize) -> usize,
    ) -> Option<(ConstraintSet<P>, GroupElementId)> {
        let mut coset = self.solve_coset_impl(&constraint_set)?;

        while !self.subgroup_orbits[coset.subgroup].subgroup.is_trivial() {
            let rhs_inv = self.group().inverse(coset.rhs);
            let lhs = self.group().inverse(coset.lhs_inv);
            let canonical_largest_orbit =
                &self.subgroup_orbits[coset.subgroup].canonical_largest_orbit;
            assert!(canonical_largest_orbit.len() > 1);

            let source_candidates = canonical_largest_orbit
                .iter()
                .map(|&p| self.action.act(rhs_inv, p));
            let source = source_candidates.min()?;

            let mut destination_candidates = canonical_largest_orbit
                .iter()
                .map(|&p| self.action.act(lhs, p))
                .collect_vec();
            let random_index = random_index(destination_candidates.len());
            let (_, &mut destination, _) = destination_candidates.select_nth_unstable(random_index);

            let new_constraint = Constraint {
                from: source,
                to: destination,
            };
            constraint_set.constraints.push(new_constraint);
            coset = self.constrain_coset(coset, new_constraint)?;
        }

        self.debug_assert_constraints(&coset, &constraint_set);

        let lhs = self.group().inverse(coset.lhs_inv);
        let single_element_in_coset = self.group().compose(lhs, coset.rhs);

        Some((constraint_set, single_element_in_coset))
    }

    /// Returns a canonical set of constraints that uniquely specifies an
    /// element, assuming the base constraints have already been applied.
    ///
    /// Returns `None` if `base_constraints` is unsatisfiable.
    ///
    /// Panics in debug mode if `element` does not satisfy `base_constraints`.
    pub fn constraints_for_element(
        &mut self,
        base_constraints: &ConstraintSet<P>,
        element: GroupElementId,
    ) -> Option<ConstraintSet<P>> {
        let mut coset = self.solve_coset_impl(&base_constraints)?;

        for c in base_constraints {
            debug_assert_eq!(self.action.act(element, c.from), c.to);
        }

        let mut additional_constraints = ConstraintSet::EMPTY;

        while !self.subgroup_orbits[coset.subgroup].subgroup.is_trivial() {
            let rhs_inv = self.group().inverse(coset.rhs);
            let canonical_largest_orbit =
                &self.subgroup_orbits[coset.subgroup].canonical_largest_orbit;
            assert!(canonical_largest_orbit.len() > 1);

            let source_candidates = canonical_largest_orbit
                .iter()
                .map(|&p| self.action.act(rhs_inv, p));
            let source = source_candidates.min()?;

            let destination = self.action.act(element, source);

            let new_constraint = Constraint {
                from: source,
                to: destination,
            };
            additional_constraints.constraints.push(new_constraint);
            coset = self.constrain_coset(coset, new_constraint)?;
        }

        self.debug_assert_constraints(&coset, &additional_constraints);

        Some(additional_constraints)
    }
}

/// [`ConjugateCoset`] with inverted left-hand side: `lhs_inv^-1 * subgroup *
/// rhs`.
///
/// Constraints are added to the coset until the subgroup contains only
/// the identity, at which point `lhs_inv^-1 * rhs` satisfies all the
/// constraints.
struct ConstrainedConjugateCoset<P> {
    fixed_points: SmallVec<[P; 8]>,
    lhs_inv: GroupElementId,
    rhs: GroupElementId,
    /// Subgroup that pointwise-stabilizes `fixed_points`.
    subgroup: SubgroupId,
}

impl<P> ConstrainedConjugateCoset<P> {
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
