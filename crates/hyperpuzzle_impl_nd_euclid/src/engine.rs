use std::sync::Arc;

use hypermath::{ApproxHashMap, Vector, pga};
use hyperpuzzle_core::prelude::*;

use crate::TwistKey;

/// Simulation data for an N-dimensional Euclidean puzzle.
///
/// This type is relatively cheap to clone.
#[derive(Debug, Clone)]
pub struct NdEuclidTwistSystemEngineData {
    /// Number of dimensions of the space.
    pub ndim: u8,

    /// Vector for each axis.
    ///
    /// The axis vector is perpendicular to all layer boundaries on the axis and
    /// is fixed by all turns on the axis.
    pub axis_vectors: Arc<PerAxis<Vector>>,
    /// Map from vector to axis; inverse of `axis_vectors`.
    pub axis_from_vector: Arc<ApproxHashMap<Vector, Axis>>,

    /// Transforation to apply to pieces for each twist.
    pub twist_transforms: Arc<PerTwist<pga::Motor>>,
    /// Map from transform to twist; inverse of `twist_transforms`.
    pub twist_from_transform: Arc<ApproxHashMap<TwistKey, Twist>>,

    /// Gizmo pole distance for each twist.
    pub gizmo_pole_distances: Arc<PerTwist<Option<f32>>>,

    /// Exports from the Hyperpuzzlescript `build` function.
    pub hps_exports: Arc<hyperpuzzlescript::Map>,
}
impl TwistSystemEngineData for NdEuclidTwistSystemEngineData {}

/// Vantage set data for an N-dimensional Euclidean puzzle.
#[derive(Debug, Clone)]
pub struct NdEuclidVantageSetEngineData {
    /// View offset
    pub view_offset: pga::Motor,
}
impl VantageSetEngineData for NdEuclidVantageSetEngineData {}
