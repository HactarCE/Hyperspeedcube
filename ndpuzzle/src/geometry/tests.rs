use super::*;

/// Carves two concentric spheres and keep the shell between them.
#[test]
fn test_non_null_concentric_spheres() {
    for ndim in 1..=6 {
        println!("Testing in {ndim}D ...");
        let mut shapes = ShapeArena::new_euclidean_cga(ndim);

        shapes.carve_sphere(vector![0.0], 2.0).unwrap();
        println!("{shapes}");
        shapes.carve_sphere(vector![0.0], -1.0).unwrap();
        println!("{shapes}");

        assert_eq!(1, shapes.roots().len());
    }
}

// Carves two identical spheres.
#[test]
fn test_identical_spheres() {
    fn assert_is_sphere(arena: &CgaShapeArena) {
        assert_eq!(1, arena.roots().len());
        let root = &arena[arena.roots()[0]];
        assert_eq!(1, root.boundary.len());
        let boundary_elem_of_root = &arena[root.boundary.iter().next().unwrap().id];
        assert_eq!(0, boundary_elem_of_root.boundary.len());
    }

    for ndim in 1..=6 {
        println!("Testing in {ndim}D ...");
        let mut shapes = ShapeArena::new_euclidean_cga(ndim);

        shapes.carve_sphere(vector![0.0], 1.0).unwrap();
        println!("{shapes}");
        assert_is_sphere(&shapes);
        let prior_shapes = shapes.clone();

        shapes.carve_sphere(vector![0.0], 1.0).unwrap();
        println!("should be same as previous:\n{shapes}");
        assert_is_sphere(&shapes);
        assert_eq!(prior_shapes.to_string(), shapes.to_string());

        shapes.carve_sphere(vector![0.0], -1.0).unwrap();
        println!("{shapes}");

        assert!(shapes.is_empty())
    }
}

// Carves three spheres such that each pair has nonempty intersection, but the
// intersection of all three is empty.
#[test]
fn test_null_triple_sphere() {
    for ndim in 1..=6 {
        println!("Testing in {ndim}D ...");
        let mut shapes = ShapeArena::new_euclidean_cga(ndim);

        shapes.carve_sphere(vector![1.0], 1.5).unwrap();
        shapes.carve_sphere(vector![-1.0], 1.5).unwrap();
        println!("After two cuts:\n{shapes}");
        shapes.carve_sphere(vector![0.0], -1.15).unwrap();
        println!("After three cuts (should be empty):\n{shapes}");

        assert!(shapes.is_empty())
    }
}

// Carves two planes and one sphere such that each pair has nonempty
// intersection, but the intersection of all three is empty.
#[test]
fn test_null_double_plane_plus_sphere() {
    // ndim must be at least 2
    for ndim in 2..6 {
        println!("Testing in {ndim}D ...");
        let mut shapes = ShapeArena::new_euclidean_cga(ndim);

        shapes.carve_plane(Vector::unit(0), -1.0).unwrap();
        println!("{shapes}");
        shapes.carve_plane(Vector::unit(1), -1.0).unwrap();
        println!("{shapes}");
        shapes.carve_sphere(vector![], 1.1).unwrap();
        println!("{shapes}");

        assert!(shapes.is_empty());
    }
}

/// Carves a cube.
#[test]
fn test_cube() {
    fn assert_is_cube(arena: &CgaShapeArena, shape: ShapeId) {
        let ndim = arena[shape].ndim().unwrap();
        let expected_boundary_elems = if ndim > 1 { 2 * ndim } else { ndim } as usize;
        assert_eq!(expected_boundary_elems, arena[shape].boundary.len());
        for boundary_elem in arena[shape].boundary.iter() {
            assert_is_cube(arena, boundary_elem.id);
        }
    }

    for ndim in 1..6 {
        println!("Testing in {ndim}D ...");
        let mut shapes = ShapeArena::new_euclidean_cga(ndim);

        for ax in 0..ndim {
            shapes.carve_plane(Vector::unit(ax), 1.0).unwrap();
            shapes.carve_plane(-Vector::unit(ax), 1.0).unwrap();
        }
        println!("{shapes}");
        assert_eq!(1, shapes.roots().len());
        assert_is_cube(&shapes, shapes.roots()[0]);
    }
}
