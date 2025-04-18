use std::sync::Arc;

use hypermath::{Vector, pga::Motor};
use hyperpuzzle_core::prelude::*;

/// Simulation data for an N-dimensional Euclidean puzzle.
pub struct NdEuclidTwistSystemEngineData {
    /// Vector for each axis.
    ///
    /// The axis vector is perpendicular to all layer boundaries on the axis and
    /// is fixed by all turns on the axis.
    pub axis_vectors: Arc<PerAxis<Vector>>,
    /// Transforation to apply to pieces for each twist.
    pub twist_transforms: Arc<PerTwist<Motor>>,
    /// Gizmo pole distance for each twist.
    pub gizmo_pole_distances: Arc<PerTwist<Option<f32>>>,
}
impl TwistSystemEngineData for NdEuclidTwistSystemEngineData {}
