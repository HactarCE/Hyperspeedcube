use hypermath::prelude::*;
use itertools::Itertools;
use test_log::test;

use super::*;

/// Carves two concentric spheres and keep the shell between them.
#[test]
fn test_non_null_concentric_spheres() {
    for ndim in 1..=6 {
        println!("Testing in {ndim}D ...");
        let mut space = Space::new(ndim).unwrap();
        let mut polytopes = AtomicPolytopeSet::from_iter([space.whole_space()]);

        let mut cut = AtomicCut::carve(space.add_sphere(vector![0.0], 2.0).unwrap());
        polytopes = space.cut_atomic_polytope_set(polytopes, &mut cut).unwrap();
        let mut cut = AtomicCut::carve(space.add_sphere(vector![0.0], -1.0).unwrap());
        polytopes = space.cut_atomic_polytope_set(polytopes, &mut cut).unwrap();

        assert_eq!(1, polytopes.len());
    }
}

// Carves two identical spheres.
#[test]
fn test_identical_spheres() {
    fn assert_is_sphere(space: &Space, polytopes: &AtomicPolytopeSet) {
        assert_eq!(1, polytopes.len());
        let polytope = polytopes.iter().next().unwrap();
        let boundary = space.boundary_of(polytope).collect_vec();
        assert_eq!(1, boundary.len());
        let boundary_elem = boundary[0];
        assert_eq!(0, space.boundary_of(boundary_elem).count());
    }

    for ndim in 1..=6 {
        println!("Testing in {ndim}D ...");
        let mut space = Space::new(ndim).unwrap();
        let mut polytopes = AtomicPolytopeSet::from_iter([space.whole_space()]);

        let mut cut = AtomicCut::carve(space.add_sphere(vector![0.0], 1.0).unwrap());
        polytopes = space.cut_atomic_polytope_set(polytopes, &mut cut).unwrap();
        assert_is_sphere(&space, &polytopes);
        let prior_polytopes = polytopes.clone();

        let mut cut = AtomicCut::carve(space.add_sphere(vector![0.0], 1.0).unwrap());
        polytopes = space.cut_atomic_polytope_set(polytopes, &mut cut).unwrap();
        assert_eq!(prior_polytopes, polytopes);
        assert_is_sphere(&space, &polytopes);

        let mut cut = AtomicCut::carve(space.add_sphere(vector![0.0], -1.0).unwrap());
        polytopes = space.cut_atomic_polytope_set(polytopes, &mut cut).unwrap();

        assert!(polytopes.is_empty())
    }
}

// Carves three spheres such that each pair has nonempty intersection, but the
// intersection of all three is empty.
#[test]
fn test_null_triple_sphere() {
    for ndim in 1..=6 {
        println!("Testing in {ndim}D ...");
        let mut space = Space::new(ndim).unwrap();
        let mut polytopes = AtomicPolytopeSet::from_iter([space.whole_space()]);

        let mut cut = AtomicCut::carve(space.add_sphere(vector![1.0], 1.5).unwrap());
        polytopes = space.cut_atomic_polytope_set(polytopes, &mut cut).unwrap();
        let mut cut = AtomicCut::carve(space.add_sphere(vector![-1.0], 1.5).unwrap());
        polytopes = space.cut_atomic_polytope_set(polytopes, &mut cut).unwrap();
        assert!(!polytopes.is_empty());
        let mut cut = AtomicCut::carve(space.add_sphere(vector![0.0], -1.15).unwrap());
        polytopes = space.cut_atomic_polytope_set(polytopes, &mut cut).unwrap();
        assert!(polytopes.is_empty());
    }
}

// Carves two planes and one sphere such that each pair has nonempty
// intersection, but the intersection of all three is empty.
#[test]
fn test_null_double_plane_plus_sphere() {
    // ndim must be at least 2
    for ndim in 2..6 {
        println!("Testing in {ndim}D ...");
        let mut space = Space::new(ndim).unwrap();
        let mut polytopes = AtomicPolytopeSet::from_iter([space.whole_space()]);

        let mut cut = AtomicCut::carve(space.add_plane(Vector::unit(0), -1.0).unwrap());
        polytopes = space.cut_atomic_polytope_set(polytopes, &mut cut).unwrap();
        let mut cut = AtomicCut::carve(space.add_plane(Vector::unit(1), -1.0).unwrap());
        polytopes = space.cut_atomic_polytope_set(polytopes, &mut cut).unwrap();
        let mut cut = AtomicCut::carve(space.add_sphere(vector![], 1.1).unwrap());
        polytopes = space.cut_atomic_polytope_set(polytopes, &mut cut).unwrap();

        assert!(polytopes.is_empty());
    }
}

fn assert_is_cube(space: &Space, polytope: AtomicPolytopeId) {
    let ndim = space.ndim_of(polytope);
    let expected_boundary_len = if ndim > 1 { 2 * ndim } else { ndim } as usize;
    let actual_boundary = space.boundary_of(polytope).collect_vec();
    assert_eq!(expected_boundary_len, actual_boundary.len());
    for boundary_elem in actual_boundary {
        assert_is_cube(space, boundary_elem.id);
    }
}

/// Carves a cube.
#[test]
fn test_cube() {
    for ndim in 1..6 {
        println!("Testing in {ndim}D ...");
        let mut space = Space::new(ndim).unwrap();
        let mut polytopes = AtomicPolytopeSet::from_iter([space.whole_space()]);

        for ax in 0..ndim {
            let mut cut = AtomicCut::carve(space.add_plane(Vector::unit(ax), 1.0).unwrap());
            polytopes = space.cut_atomic_polytope_set(polytopes, &mut cut).unwrap();
            println!(
                "{}",
                space.polytope_to_string(polytopes.iter().next().unwrap())
            );

            let mut cut = AtomicCut::carve(space.add_plane(-Vector::unit(ax), 1.0).unwrap());
            polytopes = space.cut_atomic_polytope_set(polytopes, &mut cut).unwrap();
            println!(
                "{}",
                space.polytope_to_string(polytopes.iter().next().unwrap())
            );
        }
        assert_eq!(1, polytopes.len());

        assert_is_cube(&space, polytopes.iter().next().unwrap().id);

        if ndim > 4 {
            continue;
        }

        for ax in 0..ndim {
            let mut cut = AtomicCut::slice(space.add_plane(Vector::unit(ax), 0.3).unwrap());
            polytopes = space.cut_atomic_polytope_set(polytopes, &mut cut).unwrap();
            println!("Polytopes:");
            for polytope in polytopes.iter() {
                println!("{}", space.polytope_to_string(polytope));
            }
            println!();

            let mut cut = AtomicCut::slice(space.add_plane(-Vector::unit(ax), 0.3).unwrap());
            polytopes = space.cut_atomic_polytope_set(polytopes, &mut cut).unwrap();
            println!("Polytopes:");
            for polytope in polytopes.iter() {
                println!("{}", space.polytope_to_string(polytope));
            }
            println!();
        }
        assert_eq!(3_usize.pow(ndim as _), polytopes.len());

        assert_is_cube(&space, polytopes.iter().next().unwrap().id);
    }
}
