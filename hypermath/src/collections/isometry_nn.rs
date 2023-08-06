use std::fmt;
use std::marker::PhantomData;

use ball_tree::BallTree;
use tinyset::Set64;

use crate::*;

/// Nearest-neighbors query structure for unit multivectors.
#[derive(Clone)]
pub struct MultivectorNearestNeighborsMap<K: AsMultivector, V> {
    axes: Vec<Axes>,
    inner: BallTree<MultivectorPoint, V>,
    _phantom: PhantomData<K>,
}
impl<K: AsMultivector, V> fmt::Debug for MultivectorNearestNeighborsMap<K, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MultivectorNearestNeighborsMap")
            .field("axes", &self.axes)
            .finish()
    }
}
impl<K: AsMultivector, V> Default for MultivectorNearestNeighborsMap<K, V> {
    fn default() -> Self {
        Self::new(&[], vec![])
    }
}
impl<K: AsMultivector, V> MultivectorNearestNeighborsMap<K, V> {
    /// Constructs a new nearest-neighbors map from a fixed set of multivectors.
    pub fn new(multivectors: &[K], values: Vec<V>) -> Self {
        // Get the maximal set of `Axes` that is needed to contain all the
        // multivectors.
        let all_terms = multivectors.iter().flat_map(|key| key.mv().terms());
        let unique_terms = Set64::from_iter(all_terms.map(|term| term.axes));
        let axes = Vec::from_iter(unique_terms);

        let points = multivectors
            .iter()
            .map(|m| MultivectorPoint::new(&axes, m.mv()))
            .collect();

        MultivectorNearestNeighborsMap {
            axes,
            inner: BallTree::new(points, values),
            _phantom: PhantomData,
        }
    }

    /// Returns the nearest multivector in the map, measured using spherical
    /// distance.
    pub fn nearest(&self, multivector: &Isometry) -> Option<&V> {
        // Query for `multivector`.
        let key1 = MultivectorPoint::new(&self.axes, multivector.mv());
        let result1 = self.inner.query().nn(&key1).next();
        let (_point, d1, v1) = result1?;

        // Can we get a better result by querying for `-multivector`?
        let key2 = MultivectorPoint::new(&self.axes, &-multivector.mv());
        let result2 = self.inner.query().nn_within(&key2, d1).next();
        if let Some((_point, _d2, v2)) = result2 {
            Some(v2)
        } else {
            Some(v1)
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
struct MultivectorPoint(Vec<Float>);
impl MultivectorPoint {
    fn new(axes: &[Axes], multivector: &Multivector) -> Self {
        MultivectorPoint(axes.iter().map(|&ax| multivector.mv()[ax]).collect())
    }
}

impl ball_tree::Point for MultivectorPoint {
    #[allow(clippy::unnecessary_cast)] // `Float` might not be `f64`
    fn distance(&self, other: &Self) -> f64 {
        // Technically the distance metric we care about is arccosine of the dot
        // product, but since everything is normalized to the unit hypersphere,
        // Euclidean distance works well.
        std::iter::zip(&self.0, &other.0)
            .map(|(a, b)| (a - b) * (a - b))
            .sum::<Float>() as f64
    }

    fn move_towards(&self, other: &Self, d: f64) -> Self {
        // Another reason to use Euclidean distance: this operation will give us
        // points that are not on the unit hypersphere. We could normalize it to
        // put it on the sphere, by why bother?
        let t = d / self.distance(other);
        MultivectorPoint(
            std::iter::zip(&self.0, &other.0)
                .map(|(a, b)| util::mix(a, b, t as Float))
                .collect(),
        )
    }
}
