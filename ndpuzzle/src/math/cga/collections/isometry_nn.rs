use ball_tree::BallTree;
use std::fmt;
use tinyset::Set64;

use crate::math::{cga::*, util};

/// Nearest-neighbors query structure for isometries.
pub struct IsometryNearestNeighborsMap<V> {
    axes: Vec<Axes>,
    inner: BallTree<IsometryPoint, V>,
}
impl<V> fmt::Debug for IsometryNearestNeighborsMap<V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("IsometryNearestNeighborsMap")
            .field("axes", &self.axes)
            .finish()
    }
}
impl<V> IsometryNearestNeighborsMap<V> {
    /// Constructs a new nearest-neighbors map from a fixed set of isometries.
    pub fn new(isometries: &[Isometry], values: Vec<V>) -> Self {
        // Get the maximal set of `Axes` that is needed to contain all the
        // multivectors.
        let all_terms = isometries.iter().flat_map(|key| key.mv().terms());
        let unique_terms = Set64::from_iter(all_terms.map(|term| term.axes));
        let axes = Vec::from_iter(unique_terms);

        let points = isometries
            .iter()
            .map(|iso| IsometryPoint::new(&axes, iso.mv()))
            .collect();

        IsometryNearestNeighborsMap {
            axes,
            inner: BallTree::new(points, values),
        }
    }

    /// Returns the nearest isometry in the map, measured using spherical
    /// distance.
    pub fn nearest(&self, isometry: &Isometry) -> Option<&V> {
        // Query for `isometry`.
        let key1 = IsometryPoint::new(&self.axes, isometry.mv());
        let result1 = self.inner.query().nn(&key1).next();
        let (_point, d1, v1) = result1?;

        // Can we get a better result by querying for `-isometry`?
        let key2 = IsometryPoint::new(&self.axes, &-isometry.mv());
        let result2 = self.inner.query().nn_within(&key2, d1).next();
        if let Some((_point, _d2, v2)) = result2 {
            Some(v2)
        } else {
            Some(v1)
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
struct IsometryPoint(Vec<f32>);
impl IsometryPoint {
    fn new(axes: &[Axes], isometry: &Multivector) -> Self {
        IsometryPoint(axes.iter().map(|&ax| isometry.mv()[ax]).collect())
    }
}

impl ball_tree::Point for IsometryPoint {
    fn distance(&self, other: &Self) -> f64 {
        // Technically the distance metric we care about is arccosine of the dot
        // product, but since everything is normalized to the unit hypersphere
        // Euclidean distance works well.
        std::iter::zip(&self.0, &other.0)
            .map(|(a, b)| (a - b) * (a - b))
            .sum::<f32>() as f64
    }

    fn move_towards(&self, other: &Self, d: f64) -> Self {
        // Another reason to use Euclidean distance: this operation will give us
        // points that are not on the unit hypersphere.
        let t = d / self.distance(other);
        IsometryPoint(
            std::iter::zip(&self.0, &other.0)
                .map(|(a, b)| util::mix(a, b, t as f32))
                .collect(),
        )
    }
}
