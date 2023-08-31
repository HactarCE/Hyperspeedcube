use test_log::test;

use hypermath::prelude::*;

use super::*;

/// Carves two concentric spheres and keep the shell between them.
#[test]
fn test_non_null_concentric_spheres() {
    for ndim in 1..=6 {
        println!("Testing in {ndim}D ...");
        let mut space = Space::new(ndim);
        let mut shapes = ShapeSet::from(space.whole_space());

        let cut = space.add_sphere(vector![0.0], 2.0).unwrap();
        shapes = space.carve(cut).cut_set(shapes).unwrap();
        let cut = space.add_sphere(vector![0.0], -1.0).unwrap();
        shapes = space.carve(cut).cut_set(shapes).unwrap();

        assert_eq!(1, shapes.len());
    }
}

// Carves two identical spheres.
#[test]
fn test_identical_spheres() {
    fn assert_is_sphere(space: &Space, shapes: &ShapeSet) {
        assert_eq!(1, shapes.len());
        let shape = shapes.iter().next().unwrap();
        let boundary = &space[shape.id].boundary;
        assert_eq!(1, boundary.len());
        let boundary_elem = boundary.iter().next().unwrap();
        assert_eq!(0, space[boundary_elem.id].boundary.len());
    }

    for ndim in 1..=6 {
        println!("Testing in {ndim}D ...");
        let mut space = Space::new(ndim);
        let mut shapes = ShapeSet::from(space.whole_space());

        let cut = space.add_sphere(vector![0.0], 1.0).unwrap();
        shapes = space.carve(cut).cut_set(shapes).unwrap();
        assert_is_sphere(&space, &shapes);
        let prior_shapes = shapes.clone();

        let cut = space.add_sphere(vector![0.0], 1.0).unwrap();
        shapes = space.carve(cut).cut_set(shapes).unwrap();
        assert_eq!(prior_shapes, shapes);
        assert_is_sphere(&space, &shapes);

        let cut = space.add_sphere(vector![0.0], -1.0).unwrap();
        shapes = space.carve(cut).cut_set(shapes).unwrap();

        assert!(shapes.is_empty())
    }
}

// Carves three spheres such that each pair has nonempty intersection, but the
// intersection of all three is empty.
#[test]
fn test_null_triple_sphere() {
    for ndim in 1..=6 {
        println!("Testing in {ndim}D ...");
        let mut space = Space::new(ndim);
        let mut shapes = ShapeSet::from(space.whole_space());

        let cut = space.add_sphere(vector![1.0], 1.5).unwrap();
        shapes = space.carve(cut).cut_set(shapes).unwrap();
        let cut = space.add_sphere(vector![-1.0], 1.5).unwrap();
        shapes = space.carve(cut).cut_set(shapes).unwrap();
        assert!(!shapes.is_empty());
        let cut = space.add_sphere(vector![0.0], -1.15).unwrap();
        shapes = space.carve(cut).cut_set(shapes).unwrap();
        assert!(shapes.is_empty());
    }
}

// Carves two planes and one sphere such that each pair has nonempty
// intersection, but the intersection of all three is empty.
#[test]
fn test_null_double_plane_plus_sphere() {
    // ndim must be at least 2
    for ndim in 2..6 {
        println!("Testing in {ndim}D ...");
        let mut space = Space::new(ndim);
        let mut shapes = ShapeSet::from(space.whole_space());

        let cut = space.add_plane(Vector::unit(0), -1.0).unwrap();
        shapes = space.carve(cut).cut_set(shapes).unwrap();
        let cut = space.add_plane(Vector::unit(1), -1.0).unwrap();
        shapes = space.carve(cut).cut_set(shapes).unwrap();
        let cut = space.add_sphere(vector![], 1.1).unwrap();
        shapes = space.carve(cut).cut_set(shapes).unwrap();

        assert!(shapes.is_empty());
    }
}

/// Carves a cube.
#[test]
fn test_cube() {
    fn assert_is_cube(space: &Space, shape: ShapeId) {
        let ndim = space[space[shape].manifold].ndim;
        let expected_boundary_elems = if ndim > 1 { 2 * ndim } else { ndim } as usize;
        assert_eq!(expected_boundary_elems, space[shape].boundary.len());
        for boundary_elem in space[shape].boundary.iter() {
            assert_is_cube(space, boundary_elem.id);
        }
    }

    for ndim in 1..6 {
        println!("Testing in {ndim}D ...");
        let mut space = Space::new(ndim);
        let mut shapes = ShapeSet::from(space.whole_space());

        for ax in 0..ndim {
            let cut = space.add_plane(Vector::unit(ax), 1.0).unwrap();
            shapes = space.carve(cut).cut_set(shapes).unwrap();
            println!("{}", space.shape_to_string(shapes.iter().next().unwrap()));

            let cut = space.add_plane(-Vector::unit(ax), 1.0).unwrap();
            shapes = space.carve(cut).cut_set(shapes).unwrap();
            println!("{}", space.shape_to_string(shapes.iter().next().unwrap()));
        }
        assert_eq!(1, shapes.len());

        assert_is_cube(&space, shapes.iter().next().unwrap().id);

        if ndim > 4 {
            continue;
        }

        for ax in 0..ndim {
            let cut = space.add_plane(Vector::unit(ax), 0.3).unwrap();
            shapes = space.slice(cut).cut_set(shapes).unwrap();
            println!("Shapes:");
            for shape in &shapes {
                println!("{}", space.shape_to_string(shape));
            }
            println!();

            let cut = space.add_plane(-Vector::unit(ax), 0.3).unwrap();
            shapes = space.slice(cut).cut_set(shapes).unwrap();
            println!("Shapes:");
            for shape in &shapes {
                println!("{}", space.shape_to_string(shape));
            }
            println!();
        }
        assert_eq!(3_usize.pow(ndim as _), shapes.len());

        assert_is_cube(&space, shapes.iter().next().unwrap().id);
    }
}
