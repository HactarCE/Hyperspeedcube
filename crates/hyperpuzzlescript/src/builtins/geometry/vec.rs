use hypermath::prelude::*;

use crate::{Result, Scope};

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
    ])
}
