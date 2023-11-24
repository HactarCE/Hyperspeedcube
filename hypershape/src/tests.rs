use hypermath::prelude::*;
use itertools::Itertools;

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

#[track_caller]
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

#[test]
fn test_patchwork_cube() {
    init_test_logging();

    let schlafli_indices = vec![4, 3, 3, 3, 3, 3];
    for ndim in 2..6 {
        let mut space = PatchworkSpace::new(ndim).unwrap();
        let folded_patch = space
            .add_schlafli_patch(SchlafliSymbol::from_indices(
                schlafli_indices[..ndim as usize - 1].to_owned(),
            ))
            .unwrap();
        let cut_manifold = space
            .add_manifold(Blade::ipns_plane(vector![0.0, 0.0, 1.0], 1.0).ipns_to_opns(ndim))
            .unwrap();
        let mut cut = space.slice(folded_patch, cut_manifold).unwrap();

        let initial_polytope = Polytope::from((folded_patch, space[folded_patch].polytope));
        let mut cube = space
            .cut(&mut cut, &initial_polytope)
            .unwrap()
            .to_vec()
            .into_iter()
            .filter_map(|x| x)
            .collect_vec();

        println!("HERE WE GO {ndim}");
        for p in dbg!(&cube) {
            for p in &p.components {
                for (_patch, &polytope) in &p.by_patch {
                    println!("{}", space.internal_space().polytope_to_string(polytope));
                }
            }
        }

        assert_eq!(cube.len(), 1);
        assert_eq!(cube[0].components.len(), 1);
        assert_eq!(cube[0].components[0].by_patch.len(), 1);
        let cube = cube
            .pop()
            .unwrap()
            .components
            .pop()
            .unwrap()
            .by_patch
            .into_iter()
            .next()
            .unwrap()
            .1;

        assert_is_cube(space.internal_space(), cube.id);
    }
}

fn init_test_logging() {
    // Initialize tracing
    use tracing_error::ErrorLayer;
    use tracing_subscriber::prelude::*;
    use tracing_subscriber::{fmt, EnvFilter};

    let fmt_layer = fmt::layer().with_target(false);
    let filter_layer = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new("info"))
        .unwrap();

    tracing_subscriber::registry()
        .with(filter_layer)
        .with(fmt_layer)
        .with(ErrorLayer::default())
        .init();

    // Initialize color_eyre.
    color_eyre::install().unwrap();
}

#[test]
fn test_spooky() {
    let mut space = Space::new(2).unwrap();
    let mut polytopes = AtomicPolytopeSet::new();
    polytopes.insert(space.whole_space());
    for manifold in [
        space.add_sphere(vector![3.0], -2.0).unwrap(),
        space.add_sphere(vector![-3.0], -2.0).unwrap(),
        space.add_plane(vector![0.0, 1.0], 1.0).unwrap(),
        space.add_plane(vector![0.0, -1.0], 1.0).unwrap(),
    ] {
        let mut cut = AtomicCut::carve(manifold);
        polytopes = space.cut_atomic_polytope_set(polytopes, &mut cut).unwrap();
    }
    for p in polytopes.iter() {
        println!("{}", space.polytope_to_string(p));
    }
    println!("Final cut ...");
    let manifold = space.add_sphere(vector![], 3.0).unwrap();
    let mut cut = AtomicCut::carve(manifold);
    polytopes = space.cut_atomic_polytope_set(polytopes, &mut cut).unwrap();
    for p in polytopes.iter() {
        println!("{}", space.polytope_to_string(p));
    }
    panic!()
}
