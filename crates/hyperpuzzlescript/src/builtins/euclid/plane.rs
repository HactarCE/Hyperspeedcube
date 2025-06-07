use hypermath::prelude::*;

use crate::{Error, Num, Result, Scope};

pub fn define_in(scope: &Scope) -> Result<()> {
    scope.register_builtin_functions(hps_fns![
        // Construction
        ("plane", |_, (pole, pole_span): Vector| -> Hyperplane {
            Hyperplane::from_pole(&pole).ok_or_else(|| {
                Error::bad_arg(pole, Some("plane pole cannot be zero")).at(pole_span)
            })?
        }),
        ("plane", |_, (pole, pole_span): Point| -> Hyperplane {
            Hyperplane::from_pole(&pole.0).ok_or_else(|| {
                Error::bad_arg(pole, Some("plane pole cannot be zero")).at(pole_span)
            })?
        }),
        (
            "plane",
            |_, (normal, normal_span): Vector, distance: Num| -> Hyperplane {
                Hyperplane::new(&normal, distance).ok_or_else(|| {
                    Error::bad_arg(normal, Some("plane normal vector cannot be zero"))
                        .at(normal_span)
                })?
            }
        ),
        (
            "plane",
            |_, (normal, normal_span): Vector, point: Point| -> Hyperplane {
                Hyperplane::through_point(&normal, point.0).ok_or_else(|| {
                    Error::bad_arg(normal, Some("plane normal vector cannot be zero"))
                        .at(normal_span)
                })?
            }
        ),
        // Other functions
        ("distance", |_, a: Hyperplane, b: Point| -> Num {
            a.signed_distance_to_point(&b)
        }),
    ])
}
