use std::fmt;

use ball_tree::BallTree;

use crate::{pga::Motor, *};

/// Nearest-neighbors query structure for motors in the projective geometric
/// algebra.
///
/// This structure is designed for pure rotations about the origin but should
/// give reasonable results for other motors as well.
#[derive(Clone)]
pub struct MotorNearestNeighborMap<V> {
    len: usize,
    ndim: u8,
    reflections: BallTree<pga::Motor, V>,
    nonreflections: BallTree<pga::Motor, V>,
}
impl<V> fmt::Debug for MotorNearestNeighborMap<V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MultivectorNearestNeighborsMap")
            .field("len", &self.len)
            .finish()
    }
}
impl<V> Default for MotorNearestNeighborMap<V> {
    fn default() -> Self {
        Self::new(&[], vec![])
    }
}
impl<V> MotorNearestNeighborMap<V> {
    /// Constructs a new nearest-neighbors map from a fixed set of motors.
    pub fn new(motors: &[pga::Motor], values: Vec<V>) -> Self {
        let ndim = motors.iter().map(|motor| motor.ndim()).max().unwrap_or(2);
        let mut reflections = vec![];
        let mut reflection_values = vec![];
        let mut nonreflections = vec![];
        let mut nonreflection_values = vec![];
        for (motor, value) in std::iter::zip(motors, values) {
            let m = motor.to_ndim_at_least(ndim);
            if m.is_reflection() {
                reflections.push(m);
                reflection_values.push(value)
            } else {
                nonreflections.push(m);
                nonreflection_values.push(value);
            }
        }
        MotorNearestNeighborMap {
            len: motors.len(),
            ndim,
            reflections: BallTree::new(reflections, reflection_values),
            nonreflections: BallTree::new(nonreflections, nonreflection_values),
        }
    }

    /// Returns the nearest motor in the map, measured using spherical distance.
    pub fn nearest(&self, motor: &pga::Motor) -> Option<&V> {
        let m = motor.project_non_normalized(self.ndim);

        let ball_tree = if m.is_reflection() {
            &self.reflections
        } else {
            &self.nonreflections
        };

        // Query for `m`.
        let result1 = ball_tree.query().nn(&m).next();
        let (_motor, d1, v1) = result1?;

        // Can we get a better result by querying for `-m`?
        let result2 = ball_tree.query().nn_within(&-m, d1).next();
        if let Some((_motor, d2, v2)) = result2 {
            if approx_lt(&d2, &d1) {
                Some(v2)
            } else {
                Some(v1)
            }
        } else {
            Some(v1)
        }
    }
}

impl ball_tree::Point for pga::Motor {
    #[allow(clippy::unnecessary_cast)] // `Float` might not be `f64`
    fn distance(&self, other: &Self) -> f64 {
        assert_eq!(
            (self.ndim(), self.is_reflection()),
            (other.ndim(), other.is_reflection()),
            "cannot compare distance between motors of different dimension or parity",
        );
        // Technically the distance metric we care about for rotors is the
        // arccosine of the dot product, but since everything is normalized to
        // the unit hypersphere, Euclidean distance works well.
        std::iter::zip(self.coefs(), other.coefs())
            .map(|(a, b)| (a - b) * (a - b))
            .sum::<Float>()
            .sqrt() as f64
    }

    fn move_towards(&self, other: &Self, d: f64) -> Self {
        // Another reason to use Euclidean distance: this operation will give us
        // points that are not on the unit hypersphere. We could normalize it to
        // put it on the sphere, by why bother?
        let t = d / self.distance(other);
        Motor::lerp_non_normalized(self, other, t)
            .expect("cannot compare distances between motors of different dimension or parity")
    }
}
