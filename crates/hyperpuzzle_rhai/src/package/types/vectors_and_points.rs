use hypermath::{Vector, VectorRef};

use super::*;

fn try_collect_to_vector(values: &[Dynamic]) -> Result<Vector> {
    values
        .iter()
        .map(util::try_to_float)
        .collect::<Result<Vector, _>>()
}

fn try_collect_to_point(values: &[Dynamic]) -> Result<Point> {
    try_collect_to_vector(values).map(Point)
}

pub fn register(module: &mut Module) {
    module.combine_flatten(exported_module!(rhai_mod));

    // Vector constructors
    FuncRegistration::new("vec").set_into_module(module, |x, y| try_collect_to_vector(&[x, y]));
    FuncRegistration::new("vec")
        .set_into_module(module, |x, y, z| try_collect_to_vector(&[x, y, z]));
    FuncRegistration::new("vec")
        .set_into_module(module, |x, y, z, w| try_collect_to_vector(&[x, y, z, w]));
    FuncRegistration::new("vec").set_into_module(module, |x, y, z, w, v| {
        try_collect_to_vector(&[x, y, z, w, v])
    });
    FuncRegistration::new("vec").set_into_module(module, |x, y, z, w, v, u| {
        try_collect_to_vector(&[x, y, z, w, v, u])
    });
    FuncRegistration::new("vec").set_into_module(module, |x, y, z, w, v, u, t| {
        try_collect_to_vector(&[x, y, z, w, v, u, t])
    });

    // Point constructors
    FuncRegistration::new("point").set_into_module(module, |x, y| try_collect_to_point(&[x, y]));
    FuncRegistration::new("point")
        .set_into_module(module, |x, y, z| try_collect_to_point(&[x, y, z]));
    FuncRegistration::new("point")
        .set_into_module(module, |x, y, z, w| try_collect_to_point(&[x, y, z, w]));
    FuncRegistration::new("point").set_into_module(module, |x, y, z, w, v| {
        try_collect_to_point(&[x, y, z, w, v])
    });
    FuncRegistration::new("point").set_into_module(module, |x, y, z, w, v, u| {
        try_collect_to_point(&[x, y, z, w, v, u])
    });
    FuncRegistration::new("point").set_into_module(module, |x, y, z, w, v, u, t| {
        try_collect_to_point(&[x, y, z, w, v, u, t])
    });

    for (i, c) in hypermath::AXIS_NAMES.chars().enumerate() {
        let i = i as u8;
        let name = c.to_ascii_lowercase().to_string();

        let getter = || FuncRegistration::new_getter(&name);
        let setter = || FuncRegistration::new_setter(&name);

        getter().set_into_module(module, move |v: &mut Vector| v.get(i));
        getter().set_into_module(module, move |Point(p): &mut Point| p.get(i));

        setter().set_into_module(module, move |v: &mut Vector, new_value: f64| {
            v.resize_and_set(i, new_value);
        });
        setter().set_into_module(module, move |v: &mut Vector, new_value: i64| {
            v.resize_and_set(i, new_value as f64);
        });
        setter().set_into_module(module, move |Point(p): &mut Point, new_value: f64| {
            p.resize_and_set(i, new_value);
        });
        setter().set_into_module(module, move |Point(p): &mut Point, new_value: i64| {
            p.resize_and_set(i, new_value as f64);
        });
    }

    let new_func_reg = |name| FuncRegistration::new(name).in_global_namespace();

    new_func_reg("+").set_into_module(module, |u: Vector, v: Vector| u + v);
    new_func_reg("+").set_into_module(module, |u: Point, v: Vector| Point(u.0 + v));
    new_func_reg("+").set_into_module(module, |u: Vector, v: Point| Point(u + v.0));

    new_func_reg("-").set_into_module(module, |u: Vector, v: Vector| u - v);
    new_func_reg("-").set_into_module(module, |u: Point, v: Vector| Point(u.0 - v));
    new_func_reg("-").set_into_module(module, |u: Point, v: Point| u.0 - v.0);

    new_func_reg("*").set_into_module(module, |v: Vector, scalar: f64| v * scalar);
    new_func_reg("*").set_into_module(module, |v: Vector, scalar: i64| v * scalar as f64);
    new_func_reg("*").set_into_module(module, |scalar: f64, v: Vector| v * scalar);
    new_func_reg("*").set_into_module(module, |scalar: i64, v: Vector| v * scalar as f64);

    new_func_reg("/").set_into_module(module, |v: Vector, scalar: f64| v / scalar);
    new_func_reg("/").set_into_module(module, |v: Vector, scalar: i64| v / scalar as f64);

    new_func_reg("==").set_into_module(module, |u: Vector, v: Vector| hypermath::approx_eq(&u, &v));
    new_func_reg("==").set_into_module(module, |u: Point, v: Point| {
        hypermath::approx_eq(&u.0, &v.0)
    });
    new_func_reg("!=")
        .set_into_module(module, |u: Vector, v: Vector| !hypermath::approx_eq(&u, &v));
    new_func_reg("!=").set_into_module(module, |u: Point, v: Point| {
        !hypermath::approx_eq(&u.0, &v.0)
    });
}

#[export_module]
mod rhai_mod {
    use hypermath::VectorRef;

    #[rhai_fn(name = "to_string")]
    pub fn vec_to_string(v: &mut Vector) -> String {
        format!("vec{v}")
    }
    #[rhai_fn(name = "to_string")]
    pub fn point_to_string(Point(p): &mut Point) -> String {
        format!("point{p}")
    }
    #[rhai_fn(name = "to_debug")]
    pub fn vec_to_debug(v: &mut Vector) -> String {
        v.to_string()
    }
    #[rhai_fn(name = "to_debug")]
    pub fn point_to_debug(Point(p): &mut Point) -> String {
        p.to_string()
    }

    // Functions
    pub fn cross(u: Vector, v: Vector) -> Vector {
        Vector::cross_product_3d(&u, &v)
    }
    pub fn dot(u: Vector, v: Vector) -> f64 {
        Vector::dot(&u, &v)
    }

    // Vector constructor
    #[rhai_fn(return_raw, name = "vec")]
    pub fn vec1(x: Dynamic) -> Result<Vector> {
        x.as_array_ref()
            .map(|array| try_collect_to_vector(&*array))
            .unwrap_or_else(|_| try_collect_to_vector(&[x]))
    }

    // Point constructor
    #[rhai_fn(return_raw, name = "point")]
    pub fn point1(x: Dynamic) -> Result<Point> {
        x.as_array_ref()
            .map(|array| try_collect_to_point(&*array))
            .unwrap_or_else(|_| try_collect_to_point(&[x]))
    }
}
