use hypermath::{Point, Vector, point};
use itertools::Itertools;
use rand::seq::IndexedRandom;
use rand::{Rng, RngExt, SeedableRng};

use super::*;
use crate::tests::PerTestPoint;
use crate::{
    ConjugateCoset, CoxeterMatrix, GenSeq, GeneratorId, GroupAction, GroupElementId, IsometryGroup,
    PerGenerator, SubgroupAction, orbit_geometric_with_gen_seq,
};

/// Tests the constraint solver on Coxeter group H3 (dodecahedral symmetry)
#[test]
fn test_group_element_constraint_solver_h3() -> eyre::Result<()> {
    #![allow(non_snake_case)]

    let group = CoxeterMatrix::H3().isometry_group()?;
    let chiral_group = CoxeterMatrix::H3().chiral_isometry_group()?;

    let g0 = &group.generator_motors()[GeneratorId(0)];
    let g1 = &group.generator_motors()[GeneratorId(1)];
    let g2 = &group.generator_motors()[GeneratorId(2)];
    let mut test_points = PerTestPoint::<Point>::new();

    let F = test_points.push(point![0.0, 0.0, 1.0])?;
    let U = test_points.push(g2.transform(&test_points[F]))?;
    let R = test_points.push(g1.transform(&test_points[U]))?;
    let L = test_points.push(g0.transform(&test_points[R]))?;
    let DR = test_points.push(g1.transform(&test_points[L]))?;
    let DL = test_points.push(g0.transform(&test_points[DR]))?;
    let BR = test_points.push(g2.transform(&test_points[DR]))?;
    let BL = test_points.push(g2.transform(&test_points[DL]))?;
    let PR = test_points.push(g1.transform(&test_points[BL]))?;
    let PL = test_points.push(g0.transform(&test_points[PR]))?;
    let PD = test_points.push(g1.transform(&test_points[PL]))?;
    #[expect(unused)]
    let PB = test_points.push(g2.transform(&test_points[PD]))?;

    let action = group.action_on_points(&test_points)?;
    let mut solver = ConstraintSolver::new(action.clone());
    let chiral_action = chiral_group.action_on_points(&test_points)?;
    let mut chiral_solver = ConstraintSolver::new(chiral_action.clone());

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
        let constraint_set = ConstraintSet::from(constraint_set);

        let t = std::time::Instant::now();

        let coset = solver.solve(&constraint_set).unwrap();
        assert_eq!(coset.subgroup.element_count, expected_order);
        assert_coset_satisfies_constraints(&action, &coset, &constraint_set);

        let chiral_coset = chiral_solver.solve(&constraint_set).unwrap();
        assert_eq!(chiral_coset.subgroup.element_count, expected_chiral_order);
        assert_coset_satisfies_constraints(&chiral_action, &chiral_coset, &constraint_set);

        println!(
            "Computed {} constraints in {:?}",
            constraint_set.constraints.len(),
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
            .element_count,
        1,
    );
    assert!(
        chiral_solver
            .solve(&[[U, U], [L, R], [F, F]].into())
            .is_none()
    );

    assert_eq!(solver.total_subgroup_orbit_count(), 5); // 120, 10, 5, 2, 1
    assert_eq!(chiral_solver.total_subgroup_orbit_count(), 3); // 60, 5, 1

    Ok(())
}

/// Tests the constraint solver on Coxeter group A4 (4-simplex symmetry)
#[test]
fn test_group_element_constraint_solver_a4() -> eyre::Result<()> {
    #![allow(non_snake_case)]

    let group = CoxeterMatrix::A(4)?.isometry_group()?;
    let chiral_group = CoxeterMatrix::A(4)?.chiral_isometry_group()?;
    assert_eq!(120, group.element_count());
    assert_eq!(60, chiral_group.element_count());

    let gen0 = GeneratorId(0);
    let gen1 = GeneratorId(1);
    let gen2 = GeneratorId(2);
    let gen3 = GeneratorId(3);

    let mut test_points = PerTestPoint::<Point>::new();
    let E = test_points.push(point![0.0, 0.0, 0.0, 1.0])?;
    let D = test_points.push(group.generator_motors()[gen3].transform(&test_points[E]))?;
    let C = test_points.push(group.generator_motors()[gen2].transform(&test_points[D]))?;
    let B = test_points.push(group.generator_motors()[gen1].transform(&test_points[C]))?;
    let A = test_points.push(group.generator_motors()[gen0].transform(&test_points[B]))?;

    let action = group.action_on_points(&test_points)?;
    let mut solver = ConstraintSolver::new(action.clone());
    let chiral_action = chiral_group.action_on_points(&test_points)?;
    let mut chiral_solver = ConstraintSolver::new(chiral_action.clone());

    for (constraint_set, expected_order, expected_chiral_order) in [
        ([].as_slice(), 120, 60),
        (&[[A, C], [B, D], [D, E]], 2, 1),
        (&[[A, C], [B, D], [D, E], [C, B]], 1, 1),
        (&[[A, C], [B, D], [D, E], [C, B], [E, A]], 1, 1),
    ] {
        println!("Computing {constraint_set:?} ...");
        let constraint_set = constraint_set.into();

        let t = std::time::Instant::now();

        let coset = solver.solve(&constraint_set).unwrap();
        assert_eq!(coset.subgroup.element_count, expected_order);
        assert_coset_satisfies_constraints(&action, &coset, &constraint_set);

        let chiral_coset = chiral_solver.solve(&constraint_set).unwrap();
        assert_eq!(chiral_coset.subgroup.element_count, expected_chiral_order);
        assert_coset_satisfies_constraints(&chiral_action, &chiral_coset, &constraint_set);

        println!(
            "Computed {} constraints in {:?}",
            constraint_set.constraints.len(),
            t.elapsed(),
        );
    }

    assert_eq!(solver.total_subgroup_orbit_count(), 5); // 120, 24, 6, 2, 1
    assert_eq!(chiral_solver.total_subgroup_orbit_count(), 4); // 60, 12, 3, 1

    Ok(())
}

#[test]
fn test_deterministic_random_group_element() -> eyre::Result<()> {
    #![allow(non_snake_case)]

    let original_group = CoxeterMatrix::H4().isometry_group()?;
    let initial_point = Point(Vector::unit(original_group.ndim() - 1));
    let test_points: PerTestPoint<Point> = orbit_geometric_with_gen_seq(
        &original_group
            .generator_motors()
            .into_iter()
            .map(|(g, m)| (GenSeq::new([g]), m.clone()))
            .collect_vec(),
        initial_point,
    )
    .into_iter()
    .map(|(_, _, p)| p)
    .collect();
    assert_eq!(test_points.len(), 120);

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
        let action_1 = group_1.action_on_points(&test_points)?;
        let mut solver_1 = ConstraintSolver::new(action_1.clone());

        selected_1 = (0..count)
            .map(|_| {
                solver_1
                    .select(&ConstraintSet::EMPTY, |n| select_rng_1.random_range(0..n))
                    .expect("no point satisfying constraints")
            })
            .map(|(constraint_set, elem)| {
                assert_elem_satisfies_constraints(&action_1, elem, &constraint_set);
                constraint_set
            })
            .collect_vec();
    }

    let selected_2;
    {
        let action_2 = group_2.action_on_points(&test_points)?;
        let mut solver_2 = ConstraintSolver::new(action_2.clone());

        selected_2 = (0..count)
            .map(|_| {
                solver_2
                    .select(&ConstraintSet::EMPTY, |n| select_rng_2.random_range(0..n))
                    .expect("no point satisfying constraints")
            })
            .map(|(constraint_set, elem)| {
                assert_elem_satisfies_constraints(&action_2, elem, &constraint_set);
                constraint_set
            })
            .collect_vec();
    }

    assert_eq!(selected_1, selected_2);

    Ok(())
}

#[test]
fn test_product_constraint_solver() -> eyre::Result<()> {
    #![allow(non_snake_case)]

    let ga = CoxeterMatrix::B(3)?.isometry_group()?; // cube (3D)
    let gb = CoxeterMatrix::I(6)?.isometry_group()?; // 6-gon (2D)
    let gc = CoxeterMatrix::A(4)?.isometry_group()?; // 4-simplex (4D)

    let gen0 = GeneratorId(0);
    let gen1 = GeneratorId(1);
    let gen2 = GeneratorId(2);
    let gen3 = GeneratorId(3);

    // Cube
    let mut test_points_a = PerTestPoint::<Point>::new();
    let aF = test_points_a.push(point![0.0, 0.0, 1.0])?;
    let aU = test_points_a.push(ga.generator_motors()[gen2].transform(&test_points_a[aF]))?;
    let aR = test_points_a.push(ga.generator_motors()[gen1].transform(&test_points_a[aU]))?;
    let aL = test_points_a.push(ga.generator_motors()[gen0].transform(&test_points_a[aR]))?;
    let aD = test_points_a.push(ga.generator_motors()[gen1].transform(&test_points_a[aL]))?;
    #[expect(unused)]
    let aB = test_points_a.push(ga.generator_motors()[gen2].transform(&test_points_a[aD]))?;

    // 6-gon
    let mut test_points_b = PerTestPoint::<Point>::new();
    let polygon_rot = &gb.generator_motors()[gen1] * &gb.generator_motors()[gen0];
    let mut bA = test_points_b.push(point![0.0, 1.0])?;
    let mut bB = test_points_b.push(polygon_rot.transform(&test_points_b[bA]))?;
    let mut bC = test_points_b.push(polygon_rot.transform(&test_points_b[bB]))?;
    let mut bD = test_points_b.push(polygon_rot.transform(&test_points_b[bC]))?;
    let mut bE = test_points_b.push(polygon_rot.transform(&test_points_b[bD]))?;
    let mut bF = test_points_b.push(polygon_rot.transform(&test_points_b[bE]))?;
    for p in [&mut bA, &mut bB, &mut bC, &mut bD, &mut bE, &mut bF] {
        p.0 += test_points_a.len() as u16;
    }

    // 4-simplex
    let mut test_points_c = PerTestPoint::<Point>::new();
    let mut cE = test_points_c.push(point![0.0, 0.0, 0.0, 1.0])?;
    let mut cD = test_points_c.push(gc.generator_motors()[gen3].transform(&test_points_c[cE]))?;
    let mut cC = test_points_c.push(gc.generator_motors()[gen2].transform(&test_points_c[cD]))?;
    let mut cB = test_points_c.push(gc.generator_motors()[gen1].transform(&test_points_c[cC]))?;
    let mut cA = test_points_c.push(gc.generator_motors()[gen0].transform(&test_points_c[cB]))?;
    for p in [&mut cA, &mut cB, &mut cC, &mut cD, &mut cE] {
        p.0 += test_points_a.len() as u16 + test_points_b.len() as u16;
    }

    let action = GroupAction::product(&[
        ga.action_on_points(&test_points_a)?,
        gb.action_on_points(&test_points_b)?,
        gc.action_on_points(&test_points_c)?,
    ])?;
    let mut solver = ConstraintSolver::new(action.clone());

    let constraint_set = ConstraintSet::from([]);
    let coset = solver.solve(&constraint_set).unwrap();
    assert_eq!(coset.subgroup.element_count, 48 * 12 * 120);
    assert_coset_satisfies_constraints(&action, &coset, &constraint_set);

    let constraint_set = ConstraintSet::from([[bA, bA]]);
    let coset = solver.solve(&constraint_set).unwrap();
    assert_eq!(coset.subgroup.element_count, 48 * 2 * 120);
    assert_coset_satisfies_constraints(&action, &coset, &constraint_set);

    let constraint_set = ConstraintSet::from([[aF, aR], [bC, bF], [cA, cC], [cB, cD], [cD, cE]]);
    let coset = solver.solve(&constraint_set).unwrap();
    assert_eq!(coset.subgroup.element_count, 8 * 2 * 2);
    assert_coset_satisfies_constraints(&action, &coset, &constraint_set);

    let constraint_set = ConstraintSet::from([[aF, aR], [aF, aF]]);
    assert!(solver.solve(&constraint_set).is_none());

    let constraint_set = ConstraintSet::from([[bA, cA]]);
    assert!(solver.solve(&constraint_set).is_none());

    Ok(())
}

#[test]
fn test_subgroup_constraint_solver() -> eyre::Result<()> {
    let mut rng = rand::rngs::StdRng::seed_from_u64(0xABCD);

    let group = CoxeterMatrix::B(4)?.isometry_group()?;
    let points: PerTestPoint<Point> = itertools::iproduct!(0..4, [1.0, -1.0])
        .map(|(ax, sign)| Point(Vector::unit(ax) * sign))
        .collect();
    let action = group.action_on_points(&points)?;
    let mut solver = ConstraintSolver::new(action.clone());
    for p in points.iter_keys() {
        // Construct the subgroup `stab(p)`
        let subgroup_action =
            SubgroupAction::from_subgroup_predicate(&action, |e| action.act(e, p) == p)?;
        let mut subgroup_solver = SubgroupConstraintSolver::new(subgroup_action);
        for q in points.iter_keys() {
            // Construct the conjugate subgroup `x * stab(p) * x^-1 = stab(q)`
            let conjugating_element = *solver
                .solve(&ConstraintSet::from([[p, q]]))
                .unwrap()
                .elements()
                .choose(&mut rng)
                .unwrap();
            assert_eq!(q, action.act(conjugating_element, p));
            let mut conjugate_subgroup_solver =
                ConjugateSubgroupConstraintSolver::new(conjugating_element, &mut subgroup_solver);

            // Test `select()` with no constraints
            let (mut random_constraints, random_elem) = conjugate_subgroup_solver
                .select(ConstraintSet::EMPTY, |n| rng.random_range(0..n))
                .unwrap();
            assert_eq!(q, action.act(random_elem, q));
            assert_elem_satisfies_constraints(&action, random_elem, &random_constraints);

            // Test `select()` with constraints
            random_constraints.constraints.truncate(1);
            let (selected_constraints, selected_elem) = conjugate_subgroup_solver
                .select(random_constraints.clone(), |n| rng.random_range(0..n))
                .unwrap();
            assert_elem_satisfies_constraints(&action, selected_elem, &selected_constraints);
            assert_elem_satisfies_constraints(&action, selected_elem, &random_constraints);

            let full_constraints = selected_constraints;
            let mut full_constraints_with_q_fixed = full_constraints.clone();
            full_constraints_with_q_fixed
                .constraints
                .push(Constraint::fix(q));

            let partial_constraints = random_constraints;
            let mut partial_constraints_with_q_fixed = partial_constraints.clone();
            partial_constraints_with_q_fixed
                .constraints
                .push(Constraint::fix(q));

            // Test `solve()` (fully constrained)
            let expected = solver.solve(&full_constraints_with_q_fixed).unwrap();
            let actual = conjugate_subgroup_solver.solve(full_constraints).unwrap();
            assert_coset_satisfies_constraints(&action, &actual, &full_constraints_with_q_fixed);
            let mut expected_elems = expected.elements();
            let mut actual_elems = actual.elements();
            expected_elems.sort_unstable();
            actual_elems.sort_unstable();
            assert_eq!(expected_elems, actual_elems);

            // Test `solve()` (partially constrained)
            let mut expected = solver
                .solve(&partial_constraints_with_q_fixed)
                .unwrap()
                .elements();
            let mut actual = conjugate_subgroup_solver
                .solve(partial_constraints.clone())
                .unwrap()
                .elements();
            expected.sort_unstable();
            actual.sort_unstable();
            assert_eq!(expected, actual);

            // Test `constraints_for_element()`
            let new_constraints = conjugate_subgroup_solver
                .constraints_for_element(partial_constraints.clone(), random_elem)
                .unwrap();
            assert_eq!(
                vec![random_elem],
                conjugate_subgroup_solver
                    .solve(ConstraintSet::from_iter(std::iter::chain(
                        new_constraints,
                        partial_constraints,
                    )))
                    .unwrap()
                    .elements()
            );
        }
    }

    Ok(())
}

fn shuffle_group_generators(group: &IsometryGroup, rng: &mut impl Rng) -> IsometryGroup {
    let mut generators = group
        .generator_motors()
        .iter_values()
        .cloned()
        .collect_vec();
    for _ in 0..20 {
        let i = rng.random_range(0..generators.len());
        let mut j = rng.random_range(0..generators.len() - 1);
        if j >= i {
            j += 1;
        }
        generators[i] = &generators[i] * &generators[j];
    }
    IsometryGroup::from_generators("", PerGenerator::from(generators)).unwrap()
}

#[track_caller]
fn assert_coset_satisfies_constraints<P: TypedIndex>(
    action: &GroupAction<P>,
    coset: &ConjugateCoset,
    constraint_set: &ConstraintSet<P>,
) {
    let mut coset_elements = coset.elements();
    assert_eq!(
        coset.subgroup.element_count,
        coset_elements.len(),
        "coset lied about element count"
    );

    coset_elements.sort();
    coset_elements.dedup();
    assert_eq!(
        coset.subgroup.element_count,
        coset_elements.len(),
        "coset contained duplicate elements"
    );

    let g = &coset.subgroup.overgroup;
    let conjugate_coset_elements = coset
        .subgroup
        .elements()
        .into_iter()
        .map(|e| g.compose(g.compose(coset.lhs, e), coset.rhs))
        .sorted()
        .collect_vec();
    assert_eq!(coset_elements, conjugate_coset_elements);
    let left_coset = coset.to_left_coset();
    let left_coset_elements = left_coset
        .subgroup
        .elements()
        .into_iter()
        .map(|e| g.compose(left_coset.lhs, e))
        .sorted()
        .collect_vec();
    assert_eq!(coset_elements, left_coset_elements);
    let right_coset = coset.to_right_coset();
    let right_coset_elements = right_coset
        .subgroup
        .elements()
        .into_iter()
        .map(|e| g.compose(e, right_coset.rhs))
        .sorted()
        .collect_vec();
    assert_eq!(coset_elements, right_coset_elements);

    for elem in coset.elements() {
        assert_elem_satisfies_constraints(action, elem, constraint_set);
    }
}

#[track_caller]
fn assert_elem_satisfies_constraints<P: TypedIndex>(
    action: &GroupAction<P>,
    elem: GroupElementId,
    constraint_set: &ConstraintSet<P>,
) {
    for Constraint { from, to } in constraint_set {
        assert_eq!(to, action.act(elem, from), "coset violated constraint");
    }
}
