use hypermath::prelude::*;

use crate::{Error, Result, Scope};

pub fn define_in(scope: &Scope) -> Result<()> {
    scope.register_builtin_functions([
        // Construction
        hps_fn!("plane", |(pole, pole_span): Vec| -> EuclidPlane {
            Hyperplane::from_pole(&pole).ok_or_else(|| {
                Error::bad_arg(pole, Some("plane pole cannot be zero")).at(pole_span)
            })?
        }),
        hps_fn!("plane", |(pole, pole_span): EuclidPoint| -> EuclidPlane {
            Hyperplane::from_pole(&pole.0).ok_or_else(|| {
                Error::bad_arg(pole, Some("plane pole cannot be zero")).at(pole_span)
            })?
        }),
        hps_fn!("plane", |(normal, normal_span): Vec,
                          distance: Num|
         -> EuclidPlane {
            Hyperplane::new(&normal, distance).ok_or_else(|| {
                Error::bad_arg(normal, Some("plane normal vector cannot be zero")).at(normal_span)
            })?
        }),
        hps_fn!("plane", |(normal, normal_span): Vec,
                          point: EuclidPoint|
         -> EuclidPlane {
            Hyperplane::through_point(&normal, point.0).ok_or_else(|| {
                Error::bad_arg(normal, Some("plane normal vector cannot be zero")).at(normal_span)
            })?
        }),
        // Other functions
        hps_fn!("distance", |a: EuclidPlane, b: EuclidPoint| -> Num {
            a.signed_distance_to_point(&b)
        }),
    ])
}
