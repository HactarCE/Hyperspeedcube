use std::fmt;

use super::*;
use crate::*;

#[macro_use]
mod assertions {
    use super::*;

    pub fn assert_scaling_invariant<T>(
        expected: &T,
        blade: &Blade,
        blade_to_expected: fn(&Blade) -> T,
    ) where
        T: fmt::Debug + approx::AbsDiffEq<Epsilon = Float>,
    {
        assert_approx_eq!(expected, &blade_to_expected(blade));
        assert_approx_eq!(expected, &blade_to_expected(&(blade * 3.0)));
        assert_approx_eq!(expected, &blade_to_expected(&(blade * -3.0)));
    }

    pub fn assert_scaling_invariant_sign_dependent<T>(
        expected: &T,
        blade: &Blade,
        blade_to_expected: fn(&Blade) -> T,
    ) where
        T: fmt::Debug + Clone + std::ops::Neg<Output = T> + approx::AbsDiffEq<Epsilon = Float>,
    {
        assert_approx_eq!(*expected, blade_to_expected(blade));
        assert_approx_eq!(*expected, blade_to_expected(&(blade * 3.0)));
        assert_approx_eq!(-expected.clone(), blade_to_expected(&-blade));
        assert_approx_eq!(-expected.clone(), blade_to_expected(&(blade * -3.0)));
    }

    pub fn assert_scaling_invariant_point_query(
        ipns_divider: &Blade,
        point_on: &Vector,
        outside_vector: &Vector,
    ) {
        use PointWhichSide::*;

        let point_outside = point_on + outside_vector;
        let point_inside = point_on - outside_vector;

        // Test initial
        assert_eq!(On, ipns_divider.ipns_query_point(point_on));
        assert_eq!(Outside, ipns_divider.ipns_query_point(&point_outside));
        assert_eq!(Inside, ipns_divider.ipns_query_point(&point_inside));

        // Test with the point flipped.
        assert_eq!(On, ipns_divider.ipns_query_point(&-Blade::point(point_on)));
        assert_eq!(
            Outside,
            ipns_divider.ipns_query_point(&-Blade::point(&point_outside)),
        );
        assert_eq!(
            Inside,
            ipns_divider.ipns_query_point(&-Blade::point(&point_inside)),
        );

        // Test scaled (should be same)
        assert_eq!(On, (ipns_divider * 3.0).ipns_query_point(point_on));
        assert_eq!(
            Outside,
            (ipns_divider * 3.0).ipns_query_point(&point_outside)
        );
        assert_eq!(Inside, (ipns_divider * 3.0).ipns_query_point(&point_inside));

        // Test flipped (should be opposite)
        assert_eq!(On, (-ipns_divider).ipns_query_point(point_on));
        assert_eq!(Inside, (-ipns_divider).ipns_query_point(&point_outside));
        assert_eq!(Outside, (-ipns_divider).ipns_query_point(&point_inside));
    }
}

use assertions::*;

#[test]
fn test_cga_extract_ni_no() {
    let ni = 10.0;
    let no = 3.5;
    let blade = (Multivector::NI * ni + Multivector::NO * no).grade_project(1);
    assert_eq!(ni, blade.ni());
    assert_eq!(no, blade.no());
}

#[test]
fn test_cga_vector_repr() {
    let v = vector![1.0, -2.0, 3.0];
    let blade = Blade::vector(&v);
    println!("vector = {blade}");
    assert_approx_eq!(v, blade.to_vector());
    assert_approx_eq!(Vector::EMPTY, Blade::ZERO.to_vector());
}

#[test]
fn test_cga_point_repr() {
    let v = vector![1.0, -2.0, 3.0];
    let blade = Blade::point(&v);
    println!("point = {blade}");
    assert!(blade.is_null_vector());
    assert_scaling_invariant(&v, &blade, |b| b.to_point().unwrap());
    assert_eq!(Point::Degenerate, Blade::ZERO.to_point());
}

#[test]
fn test_cga_flat_point_repr() {
    let v = vector![1.0, -2.0, 3.0];
    let blade = Blade::flat_point(&v);
    println!("flat point = {blade}");
    assert!(blade.opns_is_flat());
    for ndim in 1..=8 {
        assert!(blade.opns_is_real(), "must be real in {ndim}D");
    }
    assert_scaling_invariant(&v, &blade, |b| b.flat_point_to_point().unwrap());

    // This computation is simple enough that it should be exact in Float.
    let finite_point = Point::Finite(v);
    assert_eq!(
        [finite_point.clone(), Point::Infinity],
        blade.point_pair_to_points().unwrap(),
    );
    assert_eq!(
        [Point::Infinity, finite_point],
        (-blade).point_pair_to_points().unwrap(),
    );

    assert_eq!(Point::Degenerate, Blade::ZERO.flat_point_to_point(),);
}

#[test]
fn test_cga_ipns_sphere_repr() {
    let center = vector![1.0, -2.0, 3.0];
    let radius = 1.5;

    let blade = Blade::ipns_sphere(&center, radius);
    println!("sphere = {blade}");
    assert!(!blade.ipns_is_flat());
    assert!(blade.ipns_is_real());
    // Test center
    assert_scaling_invariant(&center, &blade, |b| b.ipns_sphere_center().unwrap());
    // Test radius
    assert_scaling_invariant_sign_dependent(&radius, &blade, |b| b.ipns_radius().unwrap());

    // Test with negative radius (inside-out)
    let blade = Blade::ipns_sphere(&center, -radius);
    assert!(!blade.ipns_is_flat());
    assert!(blade.ipns_is_real());
    println!("flipped sphere = {blade}");
    // Test center
    assert_scaling_invariant(&center, &blade, |b| b.ipns_sphere_center().unwrap());
    // Test radius
    assert_scaling_invariant_sign_dependent(&-radius, &blade, |b| b.ipns_radius().unwrap());

    // Test degenerate sphere
    assert_eq!(Point::Degenerate, Blade::ZERO.ipns_sphere_center());
    assert_eq!(None, Blade::ZERO.ipns_radius());
}

#[test]
fn test_cga_ipns_imaginary_sphere_repr() {
    let center = vector![1.0, -2.0, 3.0];
    let radius = 1.5;

    let blade = Blade::ipns_imaginary_sphere(&center, radius);
    println!("imaginary sphere = {blade}");
    assert!(!blade.ipns_is_flat());
    assert!(blade.ipns_is_imaginary());
    // Test center
    assert_scaling_invariant(&center, &blade, |b| b.ipns_sphere_center().unwrap());
    // Test radius
    assert_eq!(None, blade.ipns_radius());
}

#[test]
fn test_cga_ipns_plane_repr() {
    let pole = vector![1.0, -2.0, 3.0];
    let normal = pole.normalize().unwrap();
    let distance = pole.mag();

    let blade = Blade::ipns_plane(&pole * 42.0, pole.mag());
    println!("plane = {blade}");
    assert!(blade.ipns_is_flat());
    assert!(blade.ipns_is_real());
    // Test pole
    assert_scaling_invariant(&pole, &blade, |b| b.ipns_plane_pole());
    // Test normal
    assert_scaling_invariant_sign_dependent(&normal, &blade, |b| b.ipns_plane_normal().unwrap());
    // Test distance
    assert_scaling_invariant_sign_dependent(&distance, &blade, |b| {
        b.ipns_plane_distance().unwrap()
    });

    // Test with negative distance (facing the other way)
    let blade = Blade::ipns_plane(&pole * 42.0, -pole.mag());
    println!("flipped plane = {blade}");
    assert!(blade.ipns_is_flat());
    assert!(blade.ipns_is_real());
    // Test pole
    assert_scaling_invariant(&-&pole, &blade, |b| b.ipns_plane_pole());
    // Test normal
    assert_scaling_invariant_sign_dependent(&normal, &blade, |b| b.ipns_plane_normal().unwrap());
    // Test distance
    assert_scaling_invariant_sign_dependent(&-distance, &blade, |b| {
        b.ipns_plane_distance().unwrap()
    });

    // Test with negative normal and distance
    let blade = Blade::ipns_plane(&pole * -42.0, -pole.mag());
    println!("flipped plane = {blade}");
    assert!(blade.ipns_is_flat());
    assert!(blade.ipns_is_real());
    // Test pole
    assert_scaling_invariant(&pole, &blade, |b| b.ipns_plane_pole());
    // Test normal
    assert_scaling_invariant_sign_dependent(&-&normal, &blade, |b| b.ipns_plane_normal().unwrap());
    // Test distance
    assert_scaling_invariant_sign_dependent(&-distance, &blade, |b| {
        b.ipns_plane_distance().unwrap()
    });

    // Test degenerate plane
    assert_eq!(Blade::ZERO, Blade::ipns_plane(Vector::EMPTY, distance));
    assert_eq!(Vector::EMPTY, Blade::ZERO.ipns_plane_pole());
    assert_eq!(None, Blade::ZERO.ipns_plane_normal());
    assert_eq!(None, Blade::ZERO.ipns_plane_distance());
}

#[test]
fn test_cga_which_side_hyperplane() {
    use PointWhichSide::*;

    let plane = Blade::ipns_plane(vector![3.0, 4.0], 5.0);
    let on = vector![3.0, 4.0];
    let out_vector = vector![1.0];
    assert_scaling_invariant_point_query(&plane, &on, &out_vector);

    // Test point at infinity.
    assert_eq!(On, plane.ipns_query_point(&Blade::NI));
}

#[test]
fn test_cga_which_side_hypersphere() {
    use PointWhichSide::*;

    let sphere = Blade::ipns_sphere(vector![6.0, -8.0], 5.0);
    let on = vector![3.0, -4.0];
    let out_vector = vector![-1.0];
    assert_scaling_invariant_point_query(&sphere, &on, &out_vector);

    // Test point at infinty
    assert_eq!(Outside, sphere.ipns_query_point(&Blade::NI));
    assert_eq!(Inside, (-sphere).ipns_query_point(&Blade::NI));
}

#[test]
fn test_cga_flat_point() {
    let p = vector![1.0, -2.0, 3.0];

    let flat_point = Blade::flat_point(&p);
    assert_eq!(Some(0), flat_point.cga_opns_ndim());

    assert_approx_eq!(p, flat_point.flat_point_to_point().unwrap());

    assert!(flat_point.opns_is_flat());
    for ndim in 1..=8 {
        assert!(flat_point.opns_is_real(), "must be real in {ndim}D");
    }
}

#[test]
fn test_cga_point_pair() {
    let p1 = vector![6.0];
    let p2 = vector![-2.0, 5.0];

    let pair = Blade::point(&p1) ^ Blade::point(&p2);
    assert_eq!(Some(0), pair.cga_opns_ndim());

    let [a, b] = pair.point_pair_to_points().unwrap();
    assert_approx_eq!(p1, a.unwrap());
    assert_approx_eq!(p2, b.unwrap());

    assert!(!pair.opns_is_flat());
    for ndim in 1..=8 {
        assert!(pair.opns_is_real(), "must be real in {ndim}D");
    }
}

#[test]
fn test_cga_opns_sphere() {
    let first_point = Blade::point(vector![-1.0]);
    for ndim in 1..=8 {
        println!("In {ndim}D ...");

        let ipns_sphere_expected = Blade::ipns_sphere(vector![], 1.0);
        println!("expected IPNS sphere = {ipns_sphere_expected}");

        let opns_sphere = (0..ndim)
            .map(Vector::unit)
            .map(Blade::point)
            .fold(first_point.clone(), |a, b| a ^ b);
        println!("OPNS sphere = {opns_sphere}");

        let ipns_sphere = opns_sphere.opns_to_ipns(ndim);
        println!("IPNS sphere = {ipns_sphere}");
        let scale_factor = ipns_sphere_expected.scale_factor_to(&ipns_sphere);
        println!("Scale factor = {scale_factor:?}");

        assert!(!opns_sphere.opns_is_flat());
        assert!(opns_sphere.opns_is_real());

        assert!(scale_factor.unwrap() > 0.0);
        assert_approx_eq!(vector![], ipns_sphere.ipns_sphere_center().unwrap());
        assert_approx_eq!(1.0, ipns_sphere.ipns_radius().unwrap());
        // opns_to_ipns() and ipns_to_opns() should be inverses.
        assert_approx_eq!(opns_sphere, ipns_sphere.ipns_to_opns(ndim));

        println!();
    }
}

#[test]
fn test_cga_opns_plane() {
    let first_point = Blade::point(vector![3.0]);
    for ndim in 1..=8 {
        println!("In {ndim}D ...");

        let ipns_plane_expected = Blade::ipns_plane(vector![1.0], 3.0);
        println!("expected IPNS plane = {ipns_plane_expected}");

        let opns_plane = (1..ndim)
            .map(|i| Vector::unit(i) + vector![3.0])
            .map(Blade::point)
            .fold(Blade::NI ^ &first_point, |a, b| a ^ b);
        println!("OPNS plane = {opns_plane}");

        let ipns_plane = opns_plane.opns_to_ipns(ndim);
        println!("IPNS plane = {ipns_plane}");
        let scale_factor = ipns_plane_expected.scale_factor_to(&ipns_plane);
        println!("Scale factor = {scale_factor:?}");

        assert!(opns_plane.opns_is_flat());
        assert!(opns_plane.opns_is_real());

        assert!(scale_factor.unwrap() > 0.0);
        assert_approx_eq!(vector![3.0], ipns_plane.ipns_plane_pole());
        assert_approx_eq!(vector![1.0], ipns_plane.ipns_plane_normal().unwrap());
        assert_approx_eq!(3.0, ipns_plane.ipns_plane_distance().unwrap());
        // opns_to_ipns() and ipns_to_opns() should be inverses.
        assert_approx_eq!(opns_plane, ipns_plane.ipns_to_opns(ndim));

        println!();
    }
}

#[test]
fn test_cga_ipns_reflect_points() {
    fn reflect_thru_sphere(center: &Vector, radius: Float, point: &Vector) -> Vector {
        let vector_from_center = point - center;
        let r2 = radius * radius;
        center + &vector_from_center * (r2 / vector_from_center.mag2())
    }
    fn reflect_thru_plane(normal: &Vector, distance: Float, point: &Vector) -> Vector {
        let normal = normal.normalize().unwrap();
        let point_on_plane = &normal * distance;
        let vector_to_point = point - &point_on_plane;
        let perpendicular = &normal * vector_to_point.dot(&normal);
        let parallel = vector_to_point - &perpendicular;
        point_on_plane + parallel - &perpendicular
    }

    let center = vector![1.0, 2.0];
    let radius = 3.0;
    let plane_normal = vector![-1.0, 2.0];
    let plane_distance = 6.0;

    let sphere = Blade::ipns_sphere(&center, radius);
    let plane = Blade::ipns_plane(&plane_normal, plane_distance);

    let p1 = vector![-1.5, 4.5];
    let p2 = vector![0.0, 8.0];

    let p1_blade = Blade::point(&p1);
    let p2_blade = Blade::point(&p2);

    assert_approx_eq!(p1, p1_blade.to_point().unwrap());
    assert_approx_eq!(p2, p2_blade.to_point().unwrap());

    // Reflect a point across a sphere.
    println!("{p1_blade}");
    println!("{}", sphere.ipns_reflect_opns(&p1_blade));
    assert_approx_eq!(
        reflect_thru_sphere(&center, radius, &p1),
        sphere.ipns_reflect_opns(&p1_blade).to_point().unwrap(),
    );

    // Reflect a point pair across a sphere.
    let [reflected_p1, reflected_p2] = sphere
        .ipns_reflect_opns(&(&p1_blade ^ &p2_blade))
        .point_pair_to_points()
        .unwrap();
    assert_approx_eq!(
        reflect_thru_sphere(&center, radius, &p1),
        reflected_p2.unwrap(),
    );
    assert_approx_eq!(
        reflect_thru_sphere(&center, radius, &p2),
        reflected_p1.unwrap(),
    );

    // Reflect a point across a plane.
    assert_approx_eq!(
        reflect_thru_plane(&plane_normal, plane_distance, &p1),
        plane.ipns_reflect_opns(&p1_blade).to_point().unwrap()
    );

    // Reflect a point pair across a plane. This swaps the order of the points,
    // which is kinda unintuitive, but necessary to maintain orientation in the
    // way we want.
    let [reflected_p1, reflected_p2] = plane
        .ipns_reflect_opns(&(&p1_blade ^ &p2_blade))
        .point_pair_to_points()
        .unwrap();
    assert_approx_eq!(
        reflect_thru_plane(&plane_normal, plane_distance, &p1),
        reflected_p2.unwrap(),
    );
    assert_approx_eq!(
        reflect_thru_plane(&plane_normal, plane_distance, &p2),
        reflected_p1.unwrap(),
    );
}

#[test]
fn test_cga_ipns_reflect_sphere() {
    use PointWhichSide::*;

    let plane = Blade::ipns_plane(vector![1.0], 0.0);
    let sphere = Blade::ipns_sphere(vector![2.0], 1.0);
    let reflected_sphere = plane.ipns_reflect_ipns(&sphere);

    let p1 = vector![2.5];
    let p2 = vector![-2.5];

    assert_eq!(Outside, sphere.ipns_query_point(vector![]));
    assert_eq!(Inside, sphere.ipns_query_point(&p1));
    assert_eq!(Outside, sphere.ipns_query_point(&p2));

    assert_eq!(Outside, reflected_sphere.ipns_query_point(vector![]));
    assert_eq!(Outside, reflected_sphere.ipns_query_point(&p1));
    assert_eq!(Inside, reflected_sphere.ipns_query_point(&p2));

    for ndim in 1..=8 {
        println!("In {ndim}D ...");

        println!("IPNS sphere = {sphere}");
        let opns_sphere = sphere.ipns_to_opns(ndim);
        println!("OPNS sphere = {opns_sphere}");
        let reflected_opns_sphere = plane.ipns_reflect_opns(&opns_sphere);
        println!("reflected OPNS sphere = {reflected_opns_sphere}");
        let reflected_sphere = reflected_opns_sphere.opns_to_ipns(ndim);
        println!("reflected IPNS sphere = {reflected_sphere}");
        let new_sphere = plane.ipns_reflect_ipns(&reflected_sphere);
        println!("IPNS sphere = {new_sphere}");
        let scale_factor = sphere.scale_factor_to(&new_sphere);
        println!("scale factor = {scale_factor:?}");
        assert!(scale_factor.unwrap() > 0.0);

        println!();
    }
}

#[test]
fn test_cga_transform_manifolds_preserves_orientation() {
    let refl = Isometry::from_reflection(vector![1.0]).unwrap();
    let rot = Isometry::from_vec_to_vec(vector![1.0], vector![0.0, 1.0]).unwrap();

    // Point
    let point = Blade::point(vector![1.0]);
    println!("Testing with manifold grade=1");
    // Reflect
    let actual = refl.transform_blade(&point);
    let expected = Blade::point(vector![-1.0]);
    assert_approx_eq!(actual, expected);
    // Rotate
    let actual = rot.transform_blade(&point);
    let expected = Blade::point(vector![0.0, 1.0]);
    assert_approx_eq!(actual, expected);

    fn wedge_axes(range: std::ops::Range<u8>) -> Blade {
        range.fold(Blade::scalar(1.0), |a, i| a ^ Blade::point(Vector::unit(i)))
    }

    #[allow(non_snake_case)]
    let E = Blade::minkowski_plane();

    for ndim in 3..=8 {
        let manifold_with_x = &E ^ wedge_axes(0..ndim - 1);
        let manifold_without_x = &E ^ wedge_axes(1..ndim);
        let grade = manifold_with_x.grade();
        println!("Testing with NDIM={ndim} (manifold grade={grade})",);

        // Reflect

        let sign_change = if ndim % 2 == 0 { 1.0 } else { -1.0 };

        let actual = refl.transform_blade(&manifold_with_x);
        let expected = &E ^ Blade::point(vector![-1.0]) ^ wedge_axes(1..ndim - 1);
        assert_approx_eq!(actual, expected * sign_change);

        let actual = refl.transform_blade(&manifold_without_x);
        let expected = &manifold_without_x;
        assert_approx_eq!(actual, expected * sign_change);

        // Rotate

        let actual = rot.transform_blade(&manifold_with_x);
        assert_approx_eq!(actual, &manifold_with_x);

        let actual = rot.transform_blade(&manifold_without_x);
        let expected = &E ^ Blade::point(vector![-1.0]) ^ wedge_axes(2..ndim);
        assert_approx_eq!(actual, expected);
    }
}

#[test]
fn test_cga_ipns_reflect_pseudoscalar() {
    let plane = Blade::ipns_plane(vector![1.0], 0.0);
    for ndim in 1..=8 {
        println!("In {ndim}D ...");

        let pss = Blade::pseudoscalar(ndim);
        println!("PSS = {pss}");
        let reflected_pss = plane.ipns_reflect_opns(&pss);
        println!("reflected PSS = {reflected_pss}");
        let scale_factor = pss.scale_factor_to(&reflected_pss);
        println!("scale factor = {scale_factor:?}");
        assert!(scale_factor.unwrap() > 0.0);

        println!();
    }
}

#[test]
fn test_cga_opns_query_point() {
    use PointWhichSide::*;

    let ipns_plane = Blade::ipns_plane(vector![1.0], 1.0);
    let ipns_sphere = Blade::ipns_sphere(vector![], 2.0);
    let ipns_circle = &ipns_plane ^ &ipns_sphere;
    let point_outside = vector![2.0];
    let point_inside = vector![-2.0];
    for ndim in 1..=8 {
        println!("In {ndim}D ...");

        let opns_sphere = ipns_sphere.ipns_to_opns(ndim);
        println!("OPNS sphere = {opns_sphere}");
        let opns_circle = ipns_circle.ipns_to_opns(ndim);
        println!("OPNS circle = {opns_circle}");
        let ipns_circle_on_sphere = opns_circle.opns_to_ipns_in_space(&opns_sphere);
        println!("IPNS circle on sphere = {ipns_circle}");
        assert_eq!(
            Outside,
            ipns_circle_on_sphere.ipns_query_point(&point_outside),
        );
        assert_eq!(
            Inside,
            ipns_circle_on_sphere.ipns_query_point(&point_inside),
        );

        println!();
    }
}

#[test]
fn test_cga_is_real() {
    for ndim in 1..=8 {
        let mut opns_obj = Blade::NI ^ Blade::point(vector![-1.0]);
        for i in 0..ndim {
            opns_obj = opns_obj ^ Blade::point(Vector::unit(i));
            let grade = opns_obj.grade();
            let is_real = opns_obj.opns_is_real();
            println!("In {ndim}D, is OPNS {grade}-blade real? {is_real}");
            assert!(is_real);

            let ipns_obj = opns_obj.opns_to_ipns(ndim);
            let grade = ipns_obj.grade();
            let is_real = ipns_obj.ipns_is_real();
            println!("In {ndim}D, is IPNS {grade}-blade real? {is_real}");
            assert!(is_real);
        }
        println!();
    }
}
