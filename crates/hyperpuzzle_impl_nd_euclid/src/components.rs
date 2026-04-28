use eyre::{Result, bail};
use hypermath::{APPROX, ApproxHashMap, Vector, approx_collections, pga};
use hyperpuzzle_core::Component;
use hyperpuzzle_core::prelude::*;

use crate::TwistKey;

/// Vantage set data for an N-dimensional Euclidean puzzle.
#[derive(Debug, Clone)]
pub struct NdEuclidViewOffset {
    /// View offset
    pub view_offset: pga::Motor,
}

impl Component<VantageSet> for NdEuclidViewOffset {}

#[derive(Debug, Clone)]
pub struct NdEuclidAxisVectors {
    /// Number of dimensions of the space.
    pub ndim: u8,
    /// Vector for each axis. Each vector is unique and nonzero.
    pub vectors_by_id: PerAxis<Vector>,
    /// Map from vector to axis.
    pub ids_by_vector: ApproxHashMap<Vector, Axis>,
    /// Gizmo pole distance for each axis, for 4D twist gizmos.
    pub gizmo_pole_distances: Option<PerAxis<f32>>, // TODO: maybe this should live elsewhere?
}

impl Component<AxisSystem> for NdEuclidAxisVectors {}

impl NdEuclidAxisVectors {
    pub fn new(ndim: u8) -> Self {
        Self {
            ndim,
            vectors_by_id: PerAxis::new(),
            ids_by_vector: ApproxHashMap::new(APPROX),
            gizmo_pole_distances: (ndim == 4).then(PerAxis::new),
        }
    }

    pub fn from_vectors(ndim: u8, vectors_by_id: PerAxis<Vector>) -> Self {
        let ids_by_vector =
            ApproxHashMap::from_iter(APPROX, vectors_by_id.iter().map(|(ax, v)| (v.clone(), ax)));
        Self {
            ndim,
            vectors_by_id,
            ids_by_vector,
            gizmo_pole_distances: None,
        }
    }

    pub fn add_axis(&mut self, vector: Vector) -> Result<Axis> {
        if APPROX.eq_zero(&vector) {
            bail!("axis vector cannot be zero")
        }
        match self.ids_by_vector.entry(vector.clone()) {
            approx_collections::hash_map::Entry::Occupied(_) => {
                bail!("axis vector is already taken")
            }
            approx_collections::hash_map::Entry::Vacant(e) => {
                let id = self.vectors_by_id.push(vector)?;
                e.insert(id);
                Ok(id)
            }
        }
    }
}

#[derive(Debug)]
pub struct NdEuclidTwistsList {
    /// Number of dimensions of the space.
    pub ndim: u8,
    /// Axis for each twist.
    pub twist_axes: PerTwist<Axis>,
    /// Transforation to apply to pieces for each twist.
    pub twist_transforms: PerTwist<pga::Motor>,
    /// Map from transform to twist; inverse of `twist_transforms`.
    pub twist_from_transform: ApproxHashMap<TwistKey, Twist>,
    /// Gizmo pole distance for each twist, for 3D and 4D twist gizmos.
    pub gizmo_pole_distances: Option<PerTwist<Option<f32>>>,
}

impl Component<TwistSystem> for NdEuclidTwistsList {}

impl NdEuclidTwistsList {
    pub fn new(ndim: u8) -> Self {
        Self {
            ndim,
            twist_axes: PerTwist::new(),
            twist_transforms: PerTwist::new(),
            twist_from_transform: ApproxHashMap::new(APPROX),
            gizmo_pole_distances: (ndim == 3 || ndim == 4).then(PerTwist::new),
        }
    }

    /// Returns an iterator over all the named twists.
    pub fn iter(&self) -> TypedIndexIter<Twist> {
        self.twist_axes.iter_keys()
    }
}

#[derive(Debug, Default)]
pub struct NamedTwistsList {
    pub names: NameSpecBiMap<Twist>,
}

impl Component<TwistSystem> for NamedTwistsList {}

impl NamedTwistsList {
    pub fn new() -> Self {
        Self::default()
    }
}
