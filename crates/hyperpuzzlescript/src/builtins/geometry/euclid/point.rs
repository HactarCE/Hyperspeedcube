use hypermath::prelude::*;

use crate::{Result, Scope};

pub fn define_in(scope: &Scope) -> Result<()> {
    scope.register_builtin_functions([
        // Construction
        hps_fn!("point", |nums: List(Num)| -> EPoint {
            Point(Vector(nums.into()))
        }),
        hps_fn!("point", || -> EPoint { point![] }),
        hps_fn!("point", |x: Num| -> EPoint { point![x] }),
        hps_fn!("point", |x: Num, y: Num| -> EPoint { point![x, y] }),
        hps_fn!("point", |x: Num, y: Num, z: Num| -> EPoint {
            point![x, y, z]
        }),
        hps_fn!("point", |x: Num, y: Num, z: Num, w: Num| -> EPoint {
            point![x, y, z, w]
        }),
        hps_fn!("point", |x: Num,
                          y: Num,
                          z: Num,
                          w: Num,
                          v: Num|
         -> EPoint { point![x, y, z, w, v] }),
        hps_fn!("point", |x: Num,
                          y: Num,
                          z: Num,
                          w: Num,
                          v: Num,
                          u: Num|
         -> EPoint { point![x, y, z, w, v, u] }),
        hps_fn!("point", |x: Num,
                          y: Num,
                          z: Num,
                          w: Num,
                          v: Num,
                          u: Num,
                          t: Num|
         -> EPoint { point![x, y, z, w, v, u, t] }),
        // Operators
        hps_fn!("+", |a: EPoint, b: Vec| -> EPoint { a + b }),
        hps_fn!("+", |a: Vec, b: EPoint| -> EPoint { b + a }),
        hps_fn!("-", |a: EPoint, b: Vec| -> EPoint { a - b }),
        hps_fn!("-", |a: EPoint, b: EPoint| -> Vec { a - b }),
        // Interpolation
        hps_fn!("lerp", |a: EPoint, b: EPoint, t: Num| -> EPoint {
            Point(hypermath::util::lerp(a.0, b.0, t.clamp(0.0, 1.0)))
        }),
        hps_fn!("lerp_unbounded", |a: EPoint, b: EPoint, t: Num| -> EPoint {
            Point(hypermath::util::lerp(a.0, b.0, t))
        }),
    ])
}
