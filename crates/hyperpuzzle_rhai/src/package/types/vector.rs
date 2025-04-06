//! Rhai Euclidean vector type.

use hypermath::{Vector, VectorRef};

use super::*;

pub fn init_engine(engine: &mut Engine) {
    engine.register_type_with_name::<Vector>("vector");
}

pub fn register(module: &mut Module) {
    // Display
    new_fn("to_string").set_into_module(module, |v: &mut Vector| format!("vec{v}"));
    new_fn("to_debug").set_into_module(module, |v: &mut Vector| format!("{v:?}"));

    // Comparison
    new_fn("==").set_into_module(module, |u: Vector, v: Vector| hypermath::approx_eq(&u, &v));
    new_fn("!=").set_into_module(module, |u: Vector, v: Vector| !hypermath::approx_eq(&u, &v));

    // Operators
    new_fn("+").set_into_module(module, |u: Vector, v: Vector| u + v);

    new_fn("-").set_into_module(module, |v: Vector| -v);
    new_fn("-").set_into_module(module, |u: Vector, v: Vector| u - v);

    new_fn("*").set_into_module(module, |v: Vector, scalar: f64| v * scalar);
    new_fn("*").set_into_module(module, |v: Vector, scalar: i64| v * scalar as f64);
    new_fn("*").set_into_module(module, |scalar: f64, v: Vector| v * scalar);
    new_fn("*").set_into_module(module, |scalar: i64, v: Vector| v * scalar as f64);

    new_fn("/").set_into_module(module, |v: Vector, scalar: f64| v / scalar);
    new_fn("/").set_into_module(module, |v: Vector, scalar: i64| v / scalar as f64);

    // Constructors
    new_fn("vec").set_into_module(module, |ctx: Ctx<'_>, x: Dynamic| -> Result<_> {
        Ok(x.as_array_ref()
            .map(|array| try_collect_to_vector(&ctx, &*array))
            .unwrap_or_else(|_| try_collect_to_vector(&ctx, &[x]))?)
    });
    new_fn("vec").set_into_module(module, |ctx: Ctx<'_>, x, y| -> Result<_> {
        Ok(try_collect_to_vector(&ctx, &[x, y])?)
    });
    new_fn("vec").set_into_module(module, |ctx: Ctx<'_>, x, y, z| -> Result<_> {
        Ok(try_collect_to_vector(&ctx, &[x, y, z])?)
    });
    new_fn("vec").set_into_module(module, |ctx: Ctx<'_>, x, y, z, w| -> Result<_> {
        Ok(try_collect_to_vector(&ctx, &[x, y, z, w])?)
    });
    new_fn("vec").set_into_module(module, |ctx: Ctx<'_>, x, y, z, w, v| -> Result<_> {
        Ok(try_collect_to_vector(&ctx, &[x, y, z, w, v])?)
    });
    new_fn("vec").set_into_module(module, |ctx: Ctx<'_>, x, y, z, w, v, u| -> Result<_> {
        Ok(try_collect_to_vector(&ctx, &[x, y, z, w, v, u])?)
    });
    new_fn("vec").set_into_module(module, |ctx: Ctx<'_>, x, y, z, w, v, u, t| -> Result<_> {
        Ok(try_collect_to_vector(&ctx, &[x, y, z, w, v, u, t])?)
    });

    // Indexing
    FuncRegistration::new_index_getter().set_into_module(module, |v: &mut Vector, i: i64| {
        v.get(i.try_into().unwrap_or(0))
    });
    FuncRegistration::new_index_setter()
        .set_into_module(module, |v: &mut Vector, i: i64, new_value: f64| {
            try_set_vector_component(v, i, new_value)
        });
    FuncRegistration::new_index_setter()
        .set_into_module(module, |v: &mut Vector, i: i64, new_value: i64| {
            try_set_vector_component(v, i, new_value as f64)
        });

    // Component getters & setters
    for (i, c) in hypermath::AXIS_NAMES.chars().enumerate() {
        let i = i as u8;
        let name = c.to_ascii_lowercase().to_string();

        let getter = || FuncRegistration::new_getter(&name);
        let setter = || FuncRegistration::new_setter(&name);

        // v.x
        getter().set_into_module(module, move |v: &mut Vector| v.get(i));
        // v.x = f64
        setter().set_into_module(module, move |v: &mut Vector, new_value: f64| {
            v.resize_and_set(i, new_value);
        });
        // v.x = i64
        setter().set_into_module(module, move |v: &mut Vector, new_value: i64| {
            v.resize_and_set(i, new_value as f64);
        });
    }
}

pub(super) fn try_collect_to_vector(
    ctx: &Ctx<'_>,
    values: &[Dynamic],
) -> Result<Vector, ConvertError> {
    values.iter().map(|v| from_rhai(ctx, v.clone())).collect()
}

pub(super) fn try_set_vector_component(vector: &mut Vector, axis: i64, new_value: f64) -> Result {
    if (0..hypermath::MAX_NDIM as i64).contains(&axis) {
        vector.resize_and_set(axis as u8, new_value);
        Ok(())
    } else {
        Err(format!("bad vector index {axis}").into())
    }
}
