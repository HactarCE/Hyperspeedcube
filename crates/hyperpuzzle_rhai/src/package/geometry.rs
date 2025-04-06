//! General geometric functions.
//!
//! See [`super::types`] for functions that operate on specific types.

use hypermath::pga::Motor;
use hypermath::{Hyperplane, Vector, VectorRef};

use super::*;
use crate::Point;

pub fn register(module: &mut Module) {
    new_fn("+").set_into_module(module, |u: Point, v: Vector| Point(u.0 + v));
    new_fn("+").set_into_module(module, |u: Vector, v: Point| Point(u + v.0));

    new_fn("-").set_into_module(module, |u: Point, v: Vector| Point(u.0 - v));
    new_fn("-").set_into_module(module, |u: Point, v: Point| u.0 - v.0);

    new_fn("cross").set_into_module(module, |u: Vector, v: Vector| u.cross_product_3d(v));
    new_fn("dot").set_into_module(module, |u: Vector, v: Vector| u.dot(v));

    new_fn("distance").set_into_module(module, |u: Point, v: Point| (u.0 - v.0).mag());
    new_fn("distance2").set_into_module(module, |u: Point, v: Point| (u.0 - v.0).mag2());
    new_fn("mag").set_into_module(module, |v: Vector| v.mag());
    new_fn("mag2").set_into_module(module, |v: Vector| v.mag2());

    new_fn("project_to")
        .set_into_module(module, |v: Vector, target: Vector| v.projected_to(&target));
    new_fn("project_to").set_into_module(module, |p: Point, target: Vector| {
        Some(Point(p.0.projected_to(&target)?))
    });
    new_fn("project_to").set_into_module(module, |v: Vector, target: Hyperplane| {
        v.rejected_from(target.normal())
    });
    new_fn("project_to").set_into_module(module, |p: Point, target: Hyperplane| {
        Some(Point(target.pole() + p.0.rejected_from(target.normal())?))
    });

    new_fn("reject_from").set_into_module(
        module,
        |ctx: Ctx<'_>, a: Dynamic, b: Dynamic| -> Result<Dynamic> {
            ctx.call_native_fn("-", (a.clone(), ctx.call_native_fn("project_to", (a, b))?))
        },
    );

    new_fn("transform").set_into_module(module, |m: &mut Motor, v: Vector| m.transform_vector(v));
    new_fn("transform").set_into_module(module, |m: &mut Motor, p: Point| {
        Point(m.transform_point(p.0))
    });
    new_fn("transform").set_into_module(module, |m: &mut Motor, h: Hyperplane| m.transform(&h));
    new_fn("transform").set_into_module(module, |m1: &mut Motor, m2: Motor| m1.transform(&m2));
}
