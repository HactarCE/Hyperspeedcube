//! Rhai Euclidean hyperplane type.

use hypermath::{Hyperplane, Point, Vector};

use super::*;

pub fn init_engine(engine: &mut Engine) {
    engine.register_type_with_name::<Hyperplane>("plane");
}

pub fn register(module: &mut Module) {
    new_fn("to_string").set_into_module(module, |h: &mut Hyperplane| {
        format!("plane(vec{}, {})", h.normal(), h.distance())
    });
    new_fn("to_debug").set_into_module(module, |h: &mut Hyperplane| format!("{h:?}"));

    new_fn("plane").set_into_module(module, |pole: Vector| -> Result<Hyperplane> {
        Ok(Hyperplane::from_pole(pole).ok_or("bad hyperplane pole")?)
    });
    new_fn("plane").set_into_module(module, |pole: Point| -> Result<Hyperplane> {
        Ok(Hyperplane::from_pole(pole.as_vector()).ok_or("bad hyperplane pole")?)
    });
    new_fn("plane").set_into_module(
        module,
        |normal: Vector, distance: f64| -> Result<Hyperplane> {
            Ok(Hyperplane::new(normal, distance).ok_or("bad hyperplane normal")?)
        },
    );
    new_fn("plane").set_into_module(
        module,
        |normal: Vector, point: Point| -> Result<Hyperplane> {
            Ok(Hyperplane::through_point(normal, point.0).ok_or("bad hyperplane normal")?)
        },
    );

    new_fn("==").set_into_module(module, |h1: Hyperplane, h2: Hyperplane| {
        hypermath::approx_eq(&h1, &h2)
    });
    new_fn("!=").set_into_module(module, |h1: Hyperplane, h2: Hyperplane| {
        !hypermath::approx_eq(&h1, &h2)
    });

    new_fn("flip").set_into_module(module, |plane: Hyperplane| plane.flip());

    FuncRegistration::new_getter("normal")
        .set_into_module(module, |plane: &mut Hyperplane| plane.normal().clone());

    FuncRegistration::new_getter("distance")
        .set_into_module(module, |plane: &mut Hyperplane| plane.distance());

    new_fn("signed_distance").set_into_module(module, |h: Hyperplane, p: Point| {
        h.signed_distance_to_point(&p)
    });
}
