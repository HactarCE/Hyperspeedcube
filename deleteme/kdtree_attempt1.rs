use crate::math::*;

/// Append-only set of points in (k-1)-dimensional real projective space,
/// represented as points on the surface of a k-dimensional hypersphere.
/// Opposite points on the hypersphere are identified.
#[derive(Debug, Clone)]
pub struct KdTreeSet<T>(KdTreeNode<T>);
impl<T> Default for KdTreeSet<T> {
    fn default() -> Self {
        Self(KdTreeMap::default())
    }
}
impl<T: KdTreeKey> KdTreeSet<T> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, value: T) -> bool {
        self.0.add(value, ()).is_some()
    }
    pub fn len(&self) -> usize {
        self.0.len()
    }
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn contains(&self, value: &T) -> bool {
        self.0.contains(value)
    }
}

/// Append-only map of points in (k-1)-dimensional real projective space,
/// represented as points on the surface of a k-dimensional hypersphere.
/// Opposite points on the hypersphere are identified.
#[derive(Debug, Clone)]
pub struct KdTreeMap<K, V>(KdTreeNode<K, V>);
impl<K, V> Default for KdTreeMap<K, V> {
    fn default() -> Self {
        Self { pairs: vec![] }
    }
}
impl<K: KdTreeKey, V> KdTreeMap<K, V> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, k: K, v: V) -> Option<(K, V)> {
        match self.index_of(&k) {
            Some(i) => {
                let old = std::mem::replace(&mut self.pairs[i], (k, v));
                Some(old)
            }
            None => {
                self.pairs.push((k, v));
                None
            }
        }
    }
    pub fn len(&self) -> usize {
        self.pairs.len()
    }
    pub fn is_empty(&self) -> bool {
        self.pairs.is_empty()
    }

    pub fn contains(&self, k: &K) -> bool {
        self.index_of(k).is_some()
    }

    pub fn get(&self, k: &K) -> Option<&(K, V)> {
        self.pairs.get(self.index_of(k)?)
    }
    pub fn nearest(&self, k: &K) -> Option<&(K, V)> {
        self.pairs.get(self.index_of_nearest(k)?)
    }

    fn index_of(&self, k: &K) -> Option<usize> {
        self.pairs
            .iter()
            .enumerate()
            .find(|(_i, (k2, _v))| approx_eq(k, k2))
            .map(|(i, _)| i)
    }
    fn index_of_nearest(&self, k: &K) -> Option<usize> {
        util::min_by_f32_key(self.pairs.iter().enumerate(), |(_i, (k2, _v))| {
            K::distance(k, k2)
        })
        .map(|(i, _)| i)
    }
}

fn half_spherical_distance2(v1: impl VectorRef, v2: impl VectorRef) -> f32 {
    let v1 = v1.normalize().expect("unable to normalize vector");
    let v2 = v2.normalize().expect("unable to normalize vector");

    // We are allowed to negate a vector if it gets us a smaller distance.
    f32::min((&v1 - &v2).mag2(), (&v1 + &v2).mag2())
}

fn min_distance_to_equator(v: impl VectorRef) -> f32 {}

pub trait KdTreeKey: AbsDiffEq<Epsilon = f32> {
    // fn first_different_elem(a: Self, b: Self) -> usize;

    fn elem(i: usize) -> Option<f32>;
}

enum KdTreeNode<K, V> {
    Leaf(K, V),
    Branch(KdTreeBranch<K, V>),
}

struct KdTreeBranch<K, V> {
    skip: Option<>
    low: Option<Box<KdTreeNode<K, V>>>,
    mid: Option<Box<KdTreeNode<K, V>>>,
    high: Option<Box<KdTreeNode<K, V>>>,
}

impl<T: KdTreeKey> KdTreeBranch<T> {
    fn search_mut(&mut self, value: &T) -> &mut Option<Box<KdTreeNode<T>>> {
        match approx_cmp(&self.value, value.elem(self.axis)) {
            std::cmp::Ordering::Less => &mut self.low,
            std::cmp::Ordering::Equal => &mut self.mid,
            std::cmp::Ordering::Greater => &mut self.high,
        }
    }

    fn search(&self, value: &T) -> Option<&KdTreeNode<T>> {
        match approx_cmp(&self.value, value.elem(self.axis)) {
            std::cmp::Ordering::Less => self.low.as_ref(),
            std::cmp::Ordering::Equal => self.mid.as_ref(),
            std::cmp::Ordering::Greater => self.high.as_ref(),
        }
    }
}

fn first_difference<K: KdTreeKey>(k1: &K, k2: &K) -> FirstDifference<K> {
    k1
}
struct FirstDifference<K: KdTreeKey> {
    axis: usize,
    midpoint: f32,
}
fn split_depth(a: f32, b: f32) -> Option<usize> {
    (a != b).then_some(|| {
        if a == 0 || b == 0 || a.signum() != b.signum() {
            return Some(0);
        }
        let a = a.abs();
        let b = b.abs();

    })
}

struct Path {
    len: u32,
    branches: u32,
}
impl Path
