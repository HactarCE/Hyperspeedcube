use super::*;
use crate::{Hyperplane, Vector, VectorRef};

#[test]
fn test_cross_product() {
    let ndim = 3;
    let a = Blade::from_vector(ndim, Vector::unit(0));
    let b = Blade::from_vector(ndim, Vector::unit(1));
    let cross = Blade::cross_product_3d(&a, &b).unwrap();
    assert_approx_eq!(cross, Blade::from_vector(ndim, Vector::unit(2)));
    assert_approx_eq!(
        Vector::unit(0).cross_product_3d(Vector::unit(1)),
        Vector::unit(2),
    );
}

#[test]
fn test_transforms() {
    // TODO: why does this test run slow? run a profiler to find out!

    for ndim in 2..=7 {
        println!("Testing NDIM={ndim}");

        // Identity
        let ident = Motor::ident(ndim);
        // Rotate from X to Y
        let rot_xy = Motor::rotation(ndim, Vector::unit(0), Vector::unit(1)).unwrap();
        // Reflect across Y=1
        let plane = Hyperplane::from_pole(Vector::unit(1)).unwrap();
        let refl_plane = Motor::plane_reflection(ndim, &plane);
        // Reflect across X axis
        let refl_x = Motor::vector_reflection(ndim, Vector::unit(0)).unwrap();
        // Reflect across Y axis
        let refl_y = Motor::vector_reflection(ndim, Vector::unit(1)).unwrap();
        // Translate by [-1, +5]
        let translate = Motor::translation(ndim, vector![-1.0, 5.0]);

        println!("  Transforming points");
        let p = vector![2.0, 3.0];
        assert_approx_eq!(ident.transform_point(&p), p);
        assert_approx_eq!(rot_xy.transform_point(&p), vector![-3.0, 2.0]);
        assert_approx_eq!(refl_plane.transform_point(&p), vector![2.0, -1.0]);
        assert_approx_eq!(refl_x.transform_point(&p), vector![-2.0, 3.0]);
        assert_approx_eq!(refl_y.transform_point(&p), vector![2.0, -3.0]);
        assert_approx_eq!(translate.transform_point(&p), vector![1.0, 8.0]);

        println!("  Transforming vectors");
        let v = vector![2.0, 3.0];
        assert_approx_eq!(ident.transform_vector(&v), v);
        assert_approx_eq!(rot_xy.transform_vector(&v), vector![-3.0, 2.0]);
        assert_approx_eq!(refl_plane.transform_vector(&v), vector![2.0, -3.0]);
        assert_approx_eq!(refl_x.transform_vector(&v), vector![-2.0, 3.0]);
        assert_approx_eq!(refl_y.transform_vector(&v), vector![2.0, -3.0]);
        assert_approx_eq!(translate.transform_vector(&v), vector![2.0, 3.0]);

        println!("  Transforming vectors with canonicalized motor");
        let v = vector![2.0, 3.0];
        assert_approx_eq!(ident.canonicalize().unwrap().transform_vector(&v), v);
        assert_approx_eq!(
            rot_xy.canonicalize().unwrap().transform_vector(&v),
            vector![-3.0, 2.0],
        );
        assert_approx_eq!(
            refl_plane.canonicalize().unwrap().transform_vector(&v),
            vector![2.0, -3.0],
        );
        assert_approx_eq!(
            refl_x.canonicalize().unwrap().transform_vector(&v),
            vector![-2.0, 3.0],
        );
        assert_approx_eq!(
            refl_y.canonicalize().unwrap().transform_vector(&v),
            vector![2.0, -3.0],
        );
        assert_approx_eq!(
            translate.canonicalize().unwrap().transform_vector(&v),
            vector![2.0, 3.0],
        );

        println!("  Transforming hyperplanes");
        let h = Hyperplane::from_pole(vector![0.0, 2.0]).unwrap();
        assert_approx_eq!(ident.transform(&h), h);
        let expected = Hyperplane::from_pole(vector![-2.0, 0.0]).unwrap();
        assert_approx_eq!(rot_xy.transform(&h), &expected);
        let expected = Hyperplane::new(vector![0.0, -1.0], 0.0).unwrap();
        assert_approx_eq!(refl_plane.transform(&h), &expected);
        let expected = Hyperplane::from_pole(vector![0.0, 2.0]).unwrap();
        assert_approx_eq!(refl_x.transform(&h), &expected);
        let expected = Hyperplane::from_pole(vector![0.0, -2.0]).unwrap();
        assert_approx_eq!(refl_y.transform(&h), &expected);
        let expected = Hyperplane::from_pole(vector![0.0, 7.0]).unwrap();
        assert_approx_eq!(translate.transform(&h), &expected);
    }
}

// TODO: test transforming things other than points

#[test]
fn test_hyperplane_construction() {
    let test_cases = [
        (2, vector![4.0, 3.0], 5.0),
        (3, vector![4.0, 3.0], 5.0),
        (4, vector![4.0, 3.0], 5.0),
        (5, vector![4.0, 3.0], 5.0),
        (6, vector![4.0, 3.0], 5.0),
        (7, vector![4.0, 3.0], 5.0),
    ];
    for (ndim, normal, distance) in test_cases {
        for ns in [1.0, -1.0] {
            for ds in [1.0, -1.0] {
                let h = Hyperplane::new(&normal * ns, distance * ds).unwrap();
                let b = Blade::from_hyperplane(ndim, &h);
                assert_approx_eq!(h, b.to_hyperplane().unwrap());
            }
        }
    }
}
