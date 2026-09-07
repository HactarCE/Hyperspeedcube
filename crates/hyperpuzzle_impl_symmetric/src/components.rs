use std::sync::Arc;

use eyre::{Result, bail};
use hypermath::{APPROX, ApproxHashMap, MotorNearestNeighborMap, Vector, approx_collections, pga};
use hyperpuzzle_core::Component;
use hyperpuzzle_core::notation::Transform;
use hyperpuzzle_core::prelude::*;

use crate::TwistKey;

/// Vantage set data for an N-dimensional Euclidean puzzle.
#[derive(Debug, Clone)]
pub struct NdEuclidViewOffset {
    /// View offset
    pub view_offset: pga::Motor,
}

impl Component<VantageSet> for NdEuclidViewOffset {}

/// [`Component`] for an [`AxisSystem`] where each axis corresponds to a
/// unique nonzero vector in N-dimensional Euclidean space.
#[derive(Debug, Clone)]
pub struct NdEuclidAxisVectors {
    /// Number of dimensions of the space.
    pub ndim: u8,
    /// Vector for each axis. Each vector is unique and nonzero.
    pub vectors_by_id: PerAxis<Vector>,
    /// Map from vector to axis.
    pub ids_by_vector: ApproxHashMap<Vector, Axis>,
}

impl Component<AxisSystem> for NdEuclidAxisVectors {}

impl NdEuclidAxisVectors {
    /// Constructs the component with no axes.
    pub fn new(ndim: u8) -> Self {
        Self {
            ndim,
            vectors_by_id: PerAxis::new(),
            ids_by_vector: ApproxHashMap::new(APPROX),
        }
    }

    /// Constructs the component with the given axis vectors.
    pub fn from_vectors(ndim: u8, vectors_by_id: PerAxis<Vector>) -> Self {
        let ids_by_vector =
            ApproxHashMap::from_iter(APPROX, vectors_by_id.iter().map(|(ax, v)| (v.clone(), ax)));
        Self {
            ndim,
            vectors_by_id,
            ids_by_vector,
        }
    }

    /// Adds an axis.
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

/// [`Component`] for a [`TwistSystem`] that contains a list of twists, each
/// with a [`pga::Motor`], an optional gizmo pole distance, and a scramble
/// multiplier.
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
    /// Multiplier for each twist, if it should appear in scrambles.
    pub scramble_max_multipliers: PerTwist<Option<Multiplier>>,
}

impl Component<TwistSystem> for NdEuclidTwistsList {}

impl NdEuclidTwistsList {
    /// Constructs the component with no twists.
    pub fn new(ndim: u8) -> Self {
        Self {
            ndim,
            twist_axes: PerTwist::new(),
            twist_transforms: PerTwist::new(),
            twist_from_transform: ApproxHashMap::new(APPROX),
            gizmo_pole_distances: (ndim == 3 || ndim == 4).then(PerTwist::new),
            scramble_max_multipliers: PerTwist::new(),
        }
    }

    /// Returns an iterator over all the named twists.
    pub fn iter(&self) -> TypedIndexIter<Twist> {
        self.twist_axes.iter_keys()
    }
}

/// [`Component`] for a [`TwistSystem`] that contains a list of twists, each
/// with a unique name.
#[derive(Debug, Default)]
pub struct NamedTwistsList {
    /// Name for each twist.
    pub names: Arc<NameSpecBiMap<Twist>>,
}

impl Component<TwistSystem> for NamedTwistsList {}

impl NamedTwistsList {
    /// Constructs the component with no twists.
    pub fn new() -> Self {
        Self::default()
    }
}

/// [`Component`] for a [`TwistSystem`] where a [`pga::Motor`] can be
/// computed from each twist.
pub struct TwistToPgaMotor {
    /// Function that returns the motor for a twist transform. It may return
    /// `None` if the twist is invalid.
    pub twist_to_pga_motor: Box<dyn Send + Sync + Fn(&Transform) -> Option<pga::Motor>>,
}

impl Component<TwistSystem> for TwistToPgaMotor {}

impl TwistToPgaMotor {
    /// Constructs the component, given a function from [`Transform`] to
    /// [`pga::Motor`].
    pub fn new(f: impl 'static + Send + Sync + Fn(&Transform) -> Option<pga::Motor>) -> Arc<Self> {
        Arc::new(Self {
            twist_to_pga_motor: Box::new(f),
        })
    }
}

/// [`Component`] for a [`TwistSystem`] where the nearest twist can be computed
/// given an [`Axis`] and a [`pga::Motor`].
pub struct PgaMotorToNearestTwist {
    /// Function that returns the nearest twist to a motor on a particular axis,
    /// or `None` if the nearest twist is the identity.
    pub get_nearest_twist: Box<dyn Send + Sync + Fn(Axis, &LayerMask, &pga::Motor) -> Option<Move>>,
}

impl Component<TwistSystem> for PgaMotorToNearestTwist {}

impl PgaMotorToNearestTwist {
    /// Constructs the component, given a function from [`Axis`], [`LayerMask`],
    /// and [`pga::Motor`] to [`Move`].
    pub fn new(
        f: impl 'static + Send + Sync + Fn(Axis, &LayerMask, &pga::Motor) -> Option<Move>,
    ) -> Arc<Self> {
        Arc::new(Self {
            get_nearest_twist: Box::new(f),
        })
    }

    /// Constructs the component, given a list of all valid twists.
    pub fn from_transforms_and_multipliers(
        axis_count: usize,
        transform_to_move: impl Iterator<Item = (Axis, pga::Motor, Move)>,
    ) -> Arc<Self> {
        let mut motors_per_axis = PerAxis::<(Vec<pga::Motor>, Vec<Move>)>::new_with_len(axis_count);
        for (axis, motor, mv) in transform_to_move {
            motors_per_axis[axis].0.push(motor);
            motors_per_axis[axis].1.push(mv);
        }
        let move_nn_map_per_axis =
            motors_per_axis.map(|_, (motors, moves)| MotorNearestNeighborMap::new(&motors, moves));
        Self::new(move |axis, layers, motor| {
            let mut mv = move_nn_map_per_axis.get(axis).ok()?.nearest(motor)?.clone();
            mv.layers = layers.into();
            Some(mv)
        })
    }
}
