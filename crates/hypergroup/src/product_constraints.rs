use std::sync::Arc;

use super::*;

pub struct ProductConstraintSolver {
    action: Arc<ProductGroupAction>,
    solvers: PerFactorGroup<ConstraintSolver>,
}

impl ProductConstraintSolver {
    pub fn new(action: Arc<ProductGroupAction>) -> Self {
        let solvers = action
            .factors()
            .map_ref(|_, factor| ConstraintSolver::new(Arc::clone(factor)));

        Self { action, solvers }
    }

    fn split_constraint_set(
        &self,
        constraint_set: &ConstraintSet,
    ) -> Option<PerFactorGroup<ConstraintSet>> {
        let mut constraint_set_for_each_factor = self.solvers.map_ref(|_, _| ConstraintSet::EMPTY);

        for &Constraint { old, new } in &constraint_set.constraints {
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
                    .constraints
                    .into_iter()
                    .map(move |Constraint { old, new }| Constraint {
                        old: self.action.ref_point_from_factor(factor, old),
                        new: self.action.ref_point_from_factor(factor, new),
                    })
            })
            .collect()
    }

    pub fn solve(
        &mut self,
        constraint_set: &ConstraintSet,
    ) -> Option<ConjugateCoset<ProductSubgroup<'_>>> {
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
        let subgroup = ProductSubgroup {
            factors: conjugate_cosets
                .iter_values()
                .map(|cc| cc.subgroup)
                .collect(),
        };
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
        for constraint in &constraint_set.constraints {
            debug_assert!(combined_constraint_set.constraints.contains(constraint));
        }

        // The combined constraint set should be satisfied by the element.
        #[cfg(debug_assertions)]
        for &Constraint { old, new } in &combined_constraint_set.constraints {
            debug_assert_eq!(new, self.action.act(combined_element, old))
        }

        Some((combined_constraint_set, combined_element))
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use hypermath::{Point, point};

    use super::*;

    #[test]
    fn test_product_constraint_solver() -> eyre::Result<()> {
        #![allow(non_snake_case)]

        let ga = FiniteCoxeterGroup::B(3).coxeter_group(None)?.group()?; // cube (3D)
        let gb = FiniteCoxeterGroup::I(6).coxeter_group(None)?.group()?; // 6-gon (2D)
        let gc = FiniteCoxeterGroup::A(4).coxeter_group(None)?.group()?; // 4-simplex (4D)

        let gen0 = GeneratorId(0);
        let gen1 = GeneratorId(1);
        let gen2 = GeneratorId(2);
        let gen3 = GeneratorId(3);

        // Cube
        let mut ref_points_a = PerRefPoint::<Point>::new();
        let aF = ref_points_a.push(point![0.0, 0.0, 1.0])?;
        let aU = ref_points_a.push(ga[gen2].transform(&ref_points_a[aF]))?;
        let aR = ref_points_a.push(ga[gen1].transform(&ref_points_a[aU]))?;
        let aL = ref_points_a.push(ga[gen0].transform(&ref_points_a[aR]))?;
        let aD = ref_points_a.push(ga[gen1].transform(&ref_points_a[aL]))?;
        let aB = ref_points_a.push(ga[gen2].transform(&ref_points_a[aD]))?;

        // 6-gon
        let mut ref_points_b = PerRefPoint::<Point>::new();
        let polygon_rot = &gb[gen1] * &gb[gen0];
        let mut bA = ref_points_b.push(point![0.0, 1.0])?;
        let mut bB = ref_points_b.push(polygon_rot.transform(&ref_points_b[bA]))?;
        let mut bC = ref_points_b.push(polygon_rot.transform(&ref_points_b[bB]))?;
        let mut bD = ref_points_b.push(polygon_rot.transform(&ref_points_b[bC]))?;
        let mut bE = ref_points_b.push(polygon_rot.transform(&ref_points_b[bD]))?;
        let mut bF = ref_points_b.push(polygon_rot.transform(&ref_points_b[bE]))?;
        for p in [&mut bA, &mut bB, &mut bC, &mut bD, &mut bE, &mut bF] {
            p.0 += ref_points_a.len() as u16;
        }

        // 4-simplex
        let mut ref_points_c = PerRefPoint::<Point>::new();
        let mut cE = ref_points_c.push(point![0.0, 0.0, 0.0, 1.0])?;
        let mut cD = ref_points_c.push(gc[gen3].transform(&ref_points_c[cE]))?;
        let mut cC = ref_points_c.push(gc[gen2].transform(&ref_points_c[cD]))?;
        let mut cB = ref_points_c.push(gc[gen1].transform(&ref_points_c[cC]))?;
        let mut cA = ref_points_c.push(gc[gen0].transform(&ref_points_c[cB]))?;
        for p in [&mut cA, &mut cB, &mut cC, &mut cD, &mut cE] {
            p.0 += ref_points_a.len() as u16 + ref_points_b.len() as u16;
        }

        let action = Arc::new(ProductGroupAction::new([
            Arc::new(ga.action_on_points(&ref_points_a)?),
            Arc::new(gb.action_on_points(&ref_points_b)?),
            Arc::new(gc.action_on_points(&ref_points_c)?),
        ]));
        let mut solver = ProductConstraintSolver::new(action);

        let coset = solver.solve(&ConstraintSet::from([])).unwrap();
        assert_eq!(coset.subgroup.element_count(), 48 * 12 * 120);

        let coset = solver.solve(&ConstraintSet::from([[bA, bA]])).unwrap();
        assert_eq!(coset.subgroup.element_count(), 48 * 2 * 120);

        let coset = solver
            .solve(&ConstraintSet::from([
                [aF, aR],
                [bC, bF],
                [cA, cC],
                [cB, cD],
                [cD, cE],
            ]))
            .unwrap();
        assert_eq!(coset.subgroup.element_count(), 8 * 2 * 2);

        assert!(
            solver
                .solve(&ConstraintSet::from([[aF, aR], [aF, aF]]))
                .is_none(),
        );

        assert!(solver.solve(&ConstraintSet::from([[bA, cA]])).is_none(),);

        Ok(())
    }
}
