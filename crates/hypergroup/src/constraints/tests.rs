use std::sync::Arc;

use hypermath::{Point, Vector, point};
use itertools::Itertools;
use rand::seq::IndexedRandom;
use rand::{Rng, RngExt, SeedableRng};

use super::*;
use crate::{
    Coxeter, GenSeq, GeneratorId, GroupAction, IsometryGroup, PerGenerator, PerRefPoint,
    orbit_geometric,
};

/// Tests the constraint solver on Coxeter group H3 (dodecahedral symmetry)
#[test]
fn test_group_element_constraint_solver_h3() -> eyre::Result<()> {
    #![allow(non_snake_case)]

    let group = crate::Coxeter::H3.isometry_group()?;
    let chiral_group = crate::Coxeter::H3.chiral_isometry_group()?;

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
    let mut chiral_solver =
        ConstraintSolver::new(Arc::new(chiral_group.action_on_points(&ref_points)?));

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

    assert_eq!(solver.total_subgroup_orbit_count(), 5); // 120, 10, 5, 2, 1
    assert_eq!(chiral_solver.total_subgroup_orbit_count(), 3); // 60, 5, 1

    Ok(())
}

/// Tests the constraint solver on Coxeter group A4 (4-simplex symmetry)
#[test]
fn test_group_element_constraint_solver_a4() -> eyre::Result<()> {
    #![allow(non_snake_case)]

    let group = crate::Coxeter::A(4).isometry_group()?;
    let chiral_group = crate::Coxeter::A(4).chiral_isometry_group()?;
    assert_eq!(120, group.element_count());
    assert_eq!(60, chiral_group.element_count());

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

    assert_eq!(solver.total_subgroup_orbit_count(), 5); // 120, 24, 6, 2, 1
    assert_eq!(chiral_solver.total_subgroup_orbit_count(), 4); // 60, 12, 3, 1

    Ok(())
}

#[test]
fn test_deterministic_random_group_element() -> eyre::Result<()> {
    #![allow(non_snake_case)]

    let original_group = crate::Coxeter::H4.isometry_group()?;
    let initial_point = Point(Vector::unit(original_group.ndim() - 1));
    let ref_points: PerRefPoint<Point> = orbit_geometric(
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
    assert_eq!(ref_points.len(), 120);

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
                    .select(&ConstraintSet::EMPTY, |mut points| {
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
                    .select(&ConstraintSet::EMPTY, |mut points| {
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

#[test]
fn test_product_constraint_solver() -> eyre::Result<()> {
    #![allow(non_snake_case)]

    let ga = Coxeter::B(3).isometry_group()?; // cube (3D)
    let gb = Coxeter::I(6).isometry_group()?; // 6-gon (2D)
    let gc = Coxeter::A(4).isometry_group()?; // 4-simplex (4D)

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
    #[expect(unused)]
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

    let action = Arc::new(GroupAction::product(&[
        ga.action_on_points(&ref_points_a)?,
        gb.action_on_points(&ref_points_b)?,
        gc.action_on_points(&ref_points_c)?,
    ])?);
    let mut solver = ConstraintSolver::new(action);

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

fn shuffle_group_generators(group: &IsometryGroup, rng: &mut impl Rng) -> IsometryGroup {
    let mut generators = group
        .generator_motors()
        .into_values()
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
