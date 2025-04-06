//! Rhai Euclidean point type.

use hypermath::VectorRef;

use super::vector::{try_collect_to_vector, try_set_vector_component};
use super::*;
use crate::Point;

pub fn init_engine(engine: &mut Engine) {
    engine.register_type_with_name::<Point>("point");
}

pub fn register(module: &mut Module) {
    // Display
    new_fn("to_string").set_into_module(module, |p: &mut Point| format!("point{}", p.0));
    new_fn("to_debug").set_into_module(module, |p: &mut Point| format!("{p:?}"));

    // Comparison
    new_fn("==").set_into_module(module, |u: Point, v: Point| {
        hypermath::approx_eq(&u.0, &v.0)
    });
    new_fn("!=").set_into_module(module, |u: Point, v: Point| {
        !hypermath::approx_eq(&u.0, &v.0)
    });

    // Constructors
    new_fn("point").set_into_module(module, |ctx: Ctx<'_>, x: Dynamic| -> Result<_> {
        Ok(x.as_array_ref()
            .map(|array| try_collect_to_point(&ctx, &*array))
            .unwrap_or_else(|_| try_collect_to_point(&ctx, &[x]))?)
    });
    new_fn("point").set_into_module(module, |ctx: Ctx<'_>, x, y| -> Result<_> {
        Ok(try_collect_to_point(&ctx, &[x, y])?)
    });
    new_fn("point").set_into_module(module, |ctx: Ctx<'_>, x, y, z| -> Result<_> {
        Ok(try_collect_to_point(&ctx, &[x, y, z])?)
    });
    new_fn("point").set_into_module(module, |ctx: Ctx<'_>, x, y, z, w| -> Result<_> {
        Ok(try_collect_to_point(&ctx, &[x, y, z, w])?)
    });
    new_fn("point").set_into_module(module, |ctx: Ctx<'_>, x, y, z, w, v| -> Result<_> {
        Ok(try_collect_to_point(&ctx, &[x, y, z, w, v])?)
    });
    new_fn("point").set_into_module(module, |ctx: Ctx<'_>, x, y, z, w, v, u| -> Result<_> {
        Ok(try_collect_to_point(&ctx, &[x, y, z, w, v, u])?)
    });
    new_fn("point").set_into_module(module, |ctx: Ctx<'_>, x, y, z, w, v, u, t| -> Result<_> {
        Ok(try_collect_to_point(&ctx, &[x, y, z, w, v, u, t])?)
    });

    // Indexing
    FuncRegistration::new_index_getter().set_into_module(module, |p: &mut Point, i: i64| {
        p.0.get(i.try_into().unwrap_or(0))
    });
    FuncRegistration::new_index_setter()
        .set_into_module(module, |p: &mut Point, i: i64, new_value: f64| {
            try_set_vector_component(&mut p.0, i, new_value)
        });
    FuncRegistration::new_index_setter().set_into_module(
        module,
        |p: &mut Point, i: i64, new_value: i64| {
            try_set_vector_component(&mut p.0, i, new_value as f64)
        },
    );

    // Component getters & setters
    for (i, c) in hypermath::AXIS_NAMES.chars().enumerate() {
        let i = i as u8;
        let name = c.to_ascii_lowercase().to_string();

        let getter = || FuncRegistration::new_getter(&name);
        let setter = || FuncRegistration::new_setter(&name);

        // p.x
        getter().set_into_module(module, move |Point(p): &mut Point| p.get(i));
        // p.x = f64
        setter().set_into_module(module, move |Point(p): &mut Point, new_value: f64| {
            p.resize_and_set(i, new_value);
        });
        // p.x = i64
        setter().set_into_module(module, move |Point(p): &mut Point, new_value: i64| {
            p.resize_and_set(i, new_value as f64);
        });
    }
}

fn try_collect_to_point(ctx: &Ctx<'_>, values: &[Dynamic]) -> Result<Point, ConvertError> {
    try_collect_to_vector(ctx, values).map(Point)
}
