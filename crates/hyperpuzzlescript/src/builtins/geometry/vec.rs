use hypermath::prelude::*;

use crate::{Error, Result, Scope};

pub fn define_in(scope: &Scope) -> Result<()> {
    scope.register_builtin_functions([
        // Construction
        hps_fn!("vec", |nums: List(Num)| -> Vec { Vector(nums.into()) }),
        hps_fn!("vec", || -> Vec { vector![] }),
        hps_fn!("vec", |x: Num| -> Vec { vector![x] }),
        hps_fn!("vec", |x: Num, y: Num| -> Vec { vector![x, y] }),
        hps_fn!("vec", |x: Num, y: Num, z: Num| -> Vec { vector![x, y, z] }),
        hps_fn!("vec", |x: Num, y: Num, z: Num, w: Num| -> Vec {
            vector![x, y, z, w]
        }),
        hps_fn!("vec", |x: Num, y: Num, z: Num, w: Num, v: Num| -> Vec {
            vector![x, y, z, w, v]
        }),
        hps_fn!("vec", |x: Num,
                        y: Num,
                        z: Num,
                        w: Num,
                        v: Num,
                        u: Num|
         -> Vec { vector![x, y, z, w, v, u] }),
        hps_fn!("vec", |x: Num,
                        y: Num,
                        z: Num,
                        w: Num,
                        v: Num,
                        u: Num,
                        t: Num|
         -> Vec { vector![x, y, z, w, v, u, t] }),
        // Operators
        hps_fn!("+", |v: Vec| -> Vec { v }),
        hps_fn!("-", |v: Vec| -> Vec { -v }),
        hps_fn!("+", |a: Vec, b: Vec| -> Vec { a + b }),
        hps_fn!("-", |a: Vec, b: Vec| -> Vec { a - b }),
        hps_fn!("*", |v: Vec, n: Num| -> Vec { v * n }),
        hps_fn!("*", |n: Num, v: Vec| -> Vec { v * n }),
        hps_fn!("/", |v: Vec, n: Num| -> Vec { v / n }),
        // Functions
        hps_fn!("dot", |a: Vec, b: Vec| -> Vec { a.dot(b) }),
        hps_fn!("cross", |(a, a_span): Vec, (b, b_span): Vec| -> Vec {
            for (v, v_span) in [(&a, a_span), (&b, b_span)] {
                if v.iter_nonzero().any(|(i, _)| i >= 3) {
                    let msg = "cross product is undefined beyond 3D";
                    return Err(Error::bad_arg(v.clone(), Some(msg)).at(v_span));
                }
            }
            a.cross_product_3d(b)
        }),
        // Interpolation
        hps_fn!("lerp", |a: Vec, b: Vec, t: Num| -> Vec {
            hypermath::util::lerp(a, b, t.clamp(0.0, 1.0))
        }),
        hps_fn!("lerp_unbounded", |a: Vec, b: Vec, t: Num| -> Vec {
            hypermath::util::lerp(a, b, t)
        }),
    ])
}
