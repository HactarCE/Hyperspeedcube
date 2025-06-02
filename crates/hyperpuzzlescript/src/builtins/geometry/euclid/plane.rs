use hypermath::prelude::*;

use crate::{Error, Result, Scope};

pub fn define_in(scope: &Scope) -> Result<()> {
    scope.register_builtin_functions([
        // Construction
        hps_fn!("plane", |(pole, pole_span): Vec| -> EPlane {
            Hyperplane::from_pole(&pole).ok_or_else(|| {
                Error::bad_arg(pole, Some("plane pole cannot be zero")).at(pole_span)
            })?
        }),
        hps_fn!("plane", |(pole, pole_span): EPoint| -> EPlane {
            Hyperplane::from_pole(&pole.0).ok_or_else(|| {
                Error::bad_arg(pole, Some("plane pole cannot be zero")).at(pole_span)
            })?
        }),
        hps_fn!("plane", |(normal, normal_span): Vec,
                          distance: Num|
         -> EPlane {
            Hyperplane::new(&normal, distance).ok_or_else(|| {
                Error::bad_arg(normal, Some("plane normal vector cannot be zero")).at(normal_span)
            })?
        }),
        hps_fn!("plane", |(normal, normal_span): Vec,
                          point: EPoint|
         -> EPlane {
            Hyperplane::through_point(&normal, point.0).ok_or_else(|| {
                Error::bad_arg(normal, Some("plane normal vector cannot be zero")).at(normal_span)
            })?
        }),
        // Other functions
        hps_fn!("distance", |a: EPlane, b: EPoint| -> Num {
            a.signed_distance_to_point(&b)
        }),
    ])
}
