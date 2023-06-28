use smallvec::{smallvec, SmallVec};
use std::{cmp::Ordering, collections::BinaryHeap};
use tinyset::Set64;

use crate::math::*;

/// Append-only k-d tree set.
///
/// This structure works best when the size of `T` is small (≤32 bytes).
#[derive(Debug, Clone)]
pub struct KdTreeSet<T>(KdTreeMap<T, ()>);
impl<T: KdTreeKey> Default for KdTreeSet<T> {
    fn default() -> Self {
        Self(KdTreeMap::default())
    }
}
impl<T: KdTreeKey> KdTreeSet<T> {
    #[optick_attr::profile]
    pub fn new() -> Self {
        Self::default()
    }

    #[optick_attr::profile]
    pub fn insert(&mut self, elem: &T) -> bool {
        self.0.insert(elem, ()).is_none()
    }
    #[optick_attr::profile]
    pub fn contains(&self, elem: &T) -> bool {
        self.0.get(elem).is_some()
    }
}

/// Append-only k-d tree structure for normalized vectors (i.e., points on the
/// unit hypersphere centered at the origin).
///
/// This structure works best when the size of `(K, V)` is small (≤32 bytes).
#[derive(Debug, Clone)]
pub struct KdTreeMap<K, V> {
    root: Vec<KdTreeEntry<K, V>>,
    radius: f32,
}
impl<K: KdTreeKey, V> Default for KdTreeMap<K, V> {
    #[optick_attr::profile]
    fn default() -> Self {
        Self {
            root: vec![],
            radius: 1.0,
        }
    }
}
impl<K: KdTreeKey, V: std::fmt::Debug> KdTreeMap<K, V> {
    #[optick_attr::profile]
    pub fn new() -> Self {
        Self::default()
    }

    #[optick_attr::profile]
    fn root_bucket(&self, axes: Set64<u32>) -> Bucket<K> {
        Bucket {
            axes,
            is_top_level: true,
            center: K::Denormalized::origin(),
            radius: self.radius,
        }
    }

    #[optick_attr::profile]
    pub fn insert(&mut self, key: &K, value: V) -> Option<V> {
        match self.entry(key) {
            Entry::Occupied(mut e) => Some(e.insert(value)),
            Entry::Vacant(e) => {
                e.insert(value);
                None
            }
        }
    }

    #[optick_attr::profile]
    pub fn entry(&mut self, key: &K) -> Entry<'_, K, V> {
        let normalizations = key.normalizations();
        let existing_key = if normalizations.len() > 1 {
            normalizations.iter().find(|k| self._get(&k).is_some())
        } else {
            None
        };
        let k = existing_key
            .or_else(|| normalizations.first())
            .unwrap_or_else(|| &key);
        self._entry(k)
    }
    #[optick_attr::profile]
    fn _entry(&mut self, key: &K) -> Entry<'_, K, V> {
        let mut b = self.root_bucket(key.nonzero_axes());
        let mut entries = &mut self.root;

        loop {
            let bucket = b.bucket_key_for(key);
            b.enter_bucket(&bucket);

            match entries.binary_search_by(|entry| entry.bucket.cmp(&bucket)) {
                Ok(i) => {
                    if let KdTreeNode::Leaf(leaf) = &mut entries[i].contents {
                        if approx_eq(&leaf.key, key) {
                            return Entry::Occupied(OccupiedEntry { entries, index: i });
                        }
                    }
                    entries = entries[i].contents.make_branch(&b);
                }
                Err(i) => {
                    return Entry::Vacant(VacantEntry {
                        entries,
                        insert_index: i,
                        bucket,
                        key: key.clone(),
                    });
                }
            };
        }
    }

    #[optick_attr::profile]
    pub fn contains_key(&self, key: &K) -> bool {
        self.get(key).is_some()
    }

    #[optick_attr::profile]
    pub fn get(&self, key: &K) -> Option<&V> {
        key.normalizations().iter().find_map(|k| self._get(k))
    }
    #[optick_attr::profile]
    fn _get(&self, key: &K) -> Option<&V> {
        let mut b = self.root_bucket(key.nonzero_axes());
        let mut entries = &self.root;

        loop {
            let bucket = b.bucket_key_for(&key);
            b.enter_bucket(&bucket);

            match entries.binary_search_by(|entry| entry.bucket.cmp(&bucket)) {
                Ok(i) => match &entries[i].contents {
                    KdTreeNode::Leaf(leaf) => {
                        return approx_eq(&leaf.key, &key).then_some(&leaf.value);
                    }
                    KdTreeNode::Branch(branch) => entries = branch,
                },
                Err(_) => return None,
            };
        }
    }

    #[optick_attr::profile]
    pub fn nearest(&self, key: &K) -> Option<(&K, &V)> {
        key.normalizations()
            .iter()
            .filter_map(|k| self._nearest(k))
            .min_by(|(d1, _), (d2, _)| f32::total_cmp(d1, d2))
            .map(|(_, leaf)| (&leaf.key, &leaf.value))
    }
    #[optick_attr::profile]
    fn _nearest(&self, key: &K) -> Option<(f32, &KdTreeLeaf<K, V>)> {
        enum QueueElem<'a, K: KdTreeKey, V> {
            Branch {
                min_distance: f32,
                bucket: Bucket<K>,
                entries: &'a Vec<KdTreeEntry<K, V>>,
            },
            Leaf {
                distance: f32,
                leaf: &'a KdTreeLeaf<K, V>,
            },
        }
        impl<K: KdTreeKey, V> QueueElem<'_, K, V> {
            #[optick_attr::profile]
            fn distance(&self) -> f32 {
                match self {
                    QueueElem::Branch { min_distance, .. } => *min_distance,
                    QueueElem::Leaf { distance, .. } => *distance,
                }
            }
        }
        impl<K: KdTreeKey, V> Ord for QueueElem<'_, K, V> {
            #[optick_attr::profile]
            fn cmp(&self, other: &Self) -> Ordering {
                // Compare backwards so that the priority queue picks the
                // *shortest* distance.
                f32::total_cmp(&other.distance(), &self.distance())
            }
        }
        impl_partial_ord_and_eq!(impl<K, V> for QueueElem<'_, K, V> where K: KdTreeKey);

        let mut queue = BinaryHeap::new();
        queue.push(QueueElem::Branch {
            min_distance: 0.0,
            bucket: self.root_bucket(key.nonzero_axes()),
            entries: &self.root,
        });

        while let Some(best) = queue.pop() {
            match best {
                QueueElem::Leaf { distance, leaf } => return Some((distance, leaf)),
                QueueElem::Branch {
                    bucket, entries, ..
                } => queue.extend(entries.iter().map(|entry| match &entry.contents {
                    KdTreeNode::Branch(entries) => {
                        let mut bucket = bucket.clone();
                        bucket.enter_bucket(&entry.bucket);
                        QueueElem::Branch {
                            min_distance: bucket.min_distance_to_key(key),
                            bucket,
                            entries,
                        }
                    }
                    KdTreeNode::Leaf(leaf) => QueueElem::Leaf {
                        distance: leaf.key.distance_to(key),
                        leaf,
                    },
                })),
            }
        }

        // No elements
        None
    }
}

#[derive(Debug)]
pub enum Entry<'a, K, V> {
    Occupied(OccupiedEntry<'a, K, V>),
    Vacant(VacantEntry<'a, K, V>),
}
impl<'a, K: KdTreeKey, V> Entry<'a, K, V> {
    #[optick_attr::profile]
    pub fn or_insert(self, default: V) -> &'a mut V {
        match self {
            Entry::Occupied(e) => e.into_mut(),
            Entry::Vacant(e) => e.insert(default),
        }
    }
}
#[derive(Debug)]
pub struct OccupiedEntry<'a, K, V> {
    entries: &'a mut Vec<KdTreeEntry<K, V>>,
    index: usize,
}
impl<'a, K: KdTreeKey, V> OccupiedEntry<'a, K, V> {
    #[optick_attr::profile]
    pub fn insert(&mut self, value: V) -> V {
        std::mem::replace(self.get_mut(), value)
    }
    #[optick_attr::profile]
    pub fn get_mut(&mut self) -> &mut V {
        &mut self.entries[self.index].contents.unwrap_leaf_mut().value
    }
    #[optick_attr::profile]
    pub fn into_mut(self) -> &'a mut V {
        &mut self.entries[self.index].contents.unwrap_leaf_mut().value
    }
}
#[derive(Debug)]
pub struct VacantEntry<'a, K, V> {
    entries: &'a mut Vec<KdTreeEntry<K, V>>,
    insert_index: usize,
    bucket: BucketKey,
    key: K,
}
impl<'a, K: KdTreeKey, V> VacantEntry<'a, K, V> {
    #[optick_attr::profile]
    pub fn insert(self, value: V) -> &'a mut V {
        let i = self.insert_index;
        let bucket = self.bucket;
        let key = self.key;

        let contents = KdTreeNode::Leaf(KdTreeLeaf { key, value });
        self.entries.insert(i, KdTreeEntry { bucket, contents });

        &mut self.entries[i].contents.unwrap_leaf_mut().value
    }

    #[optick_attr::profile]
    pub fn key(&self) -> &K {
        &self.key
    }
}

// TODO: consider creating DenseSet64<T> that is like Set64<T> but always uses a
// bitset and always starts from zero, even when stored on the stack.

/// Key for a sub-bucket.
///
/// Assume the key is in the range (-1, 1), then write its two's complement
/// binary expansion. The bucket at depth `n` is the `n`th bit of that
/// expansion, starting from the sign bit and then going from most significant
/// to least significant.
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
struct BucketKey(Set64<u32>);
impl Ord for BucketKey {
    #[optick_attr::profile]
    fn cmp(&self, other: &Self) -> Ordering {
        // It doesn't really matter exactly what ordering we use here, as
        // long as it's consistent.
        if self.0.len() != other.0.len() {
            return self.0.len().cmp(&other.0.len());
        }
        if self.0 == other.0 {
            return Ordering::Equal;
        }
        self.0.iter().cmp(other.0.iter())
    }
}
impl_partial_ord!(impl for BucketKey);

#[derive(Debug, Clone)]
struct KdTreeEntry<K, V> {
    bucket: BucketKey,
    contents: KdTreeNode<K, V>,
}
impl<K, V> Ord for KdTreeEntry<K, V> {
    #[optick_attr::profile]
    fn cmp(&self, other: &Self) -> Ordering {
        self.bucket.cmp(&other.bucket)
    }
}
impl_partial_ord_and_eq!(impl<K, V> for KdTreeEntry<K, V>);

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
struct KdTreeLeaf<K, V> {
    key: K,
    value: V,
}

#[derive(Debug, Clone)]
enum KdTreeNode<K, V> {
    Branch(Vec<KdTreeEntry<K, V>>),
    Leaf(KdTreeLeaf<K, V>),
}
impl<K, V> Default for KdTreeNode<K, V> {
    #[optick_attr::profile]
    fn default() -> Self {
        KdTreeNode::Branch(vec![])
    }
}
impl<K: KdTreeKey, V> KdTreeNode<K, V> {
    #[optick_attr::profile]
    fn make_branch(&mut self, b: &Bucket<K>) -> &mut Vec<KdTreeEntry<K, V>> {
        match self {
            KdTreeNode::Branch(branch) => branch,
            KdTreeNode::Leaf(leaf) => {
                *self = KdTreeNode::Branch(vec![KdTreeEntry {
                    bucket: b.bucket_key_for(&leaf.key),
                    contents: std::mem::take(self),
                }]);
                match self {
                    KdTreeNode::Branch(branch) => branch,
                    KdTreeNode::Leaf(_) => unreachable!(),
                }
            }
        }
    }

    #[track_caller]
    #[optick_attr::profile]
    fn unwrap_leaf_mut(&mut self) -> &mut KdTreeLeaf<K, V> {
        match self {
            KdTreeNode::Leaf(leaf) => leaf,
            _ => panic!("expected leaf node"),
        }
    }
}

#[derive(Debug, Clone)]
struct Bucket<K: KdTreeKey> {
    axes: Set64<u32>,
    is_top_level: bool,
    center: K::Denormalized,
    radius: f32,
}
impl<K: KdTreeKey> Bucket<K> {
    #[optick_attr::profile]
    fn enter_bucket(&mut self, bucket: &BucketKey) {
        self.radius *= 0.5;
        for axis in self.axes.iter() {
            self.center.offset_mut(
                axis,
                // Flip the sign if it's top-level because the top level
                // determines the sign.
                if bucket.0.contains(&axis) ^ self.is_top_level {
                    self.radius
                } else {
                    -self.radius
                },
            );
        }
        self.is_top_level = false;
    }
    #[optick_attr::profile]
    fn threshold(&self, axis: u32) -> f32 {
        if self.is_top_level {
            0.0
        } else if self.axes.contains(&axis) {
            // We're tracking this axis; use `self.center`.
            self.center.get_elem(axis)
        } else {
            // We're not keeping track of this axis. If we had
            // been, it would be `self.radius` by now.
            self.radius
        }
    }
    #[optick_attr::profile]
    fn bucket_key_for(&self, k: &K) -> BucketKey {
        BucketKey(
            k.nonzero_axes()
                .iter()
                .filter(|&i| {
                    if self.is_top_level {
                        // Return sign bit.
                        approx_lt(&k.get_elem(i), &0.0)
                    } else {
                        approx_gt(&k.get_elem(i), &self.threshold(i))
                    }
                })
                .collect(),
        )
    }

    #[optick_attr::profile]
    fn min_distance_to_key(&self, key: &K) -> f32 {
        self.axes
            .iter()
            .map(|i| {
                let diff_to_center = key.get_elem(i) - self.center.get_elem(i);
                let min_diff = f32::max(0.0, diff_to_center.abs() - self.radius);
                min_diff * min_diff
            })
            .sum()
    }
}

pub trait KdTreeKey: Clone + AbsDiffEq<Epsilon = f32> + std::fmt::Debug {
    type Denormalized: KdTreeKeyDenormalized;

    fn normalizations(&self) -> SmallVec<[Self; 2]>;
    fn as_denormalized(&self) -> &Self::Denormalized;

    fn nonzero_axes(&self) -> Set64<u32> {
        self.as_denormalized().present_axes()
    }
    fn get_elem(&self, axis: u32) -> f32 {
        self.as_denormalized().get_elem(axis)
    }

    fn distance_to(&self, other: &Self) -> f32 {
        (&self.nonzero_axes() | &other.nonzero_axes())
            .iter()
            .map(|i| {
                let diff = self.get_elem(i) - other.get_elem(i);
                diff * diff
            })
            .sum()
    }
}
pub trait KdTreeKeyDenormalized: Clone + std::fmt::Debug {
    fn origin() -> Self;
    fn offset_mut(&mut self, axis: u32, amount: f32);

    fn present_axes(&self) -> Set64<u32>;
    fn get_elem(&self, axis: u32) -> f32;
}

impl KdTreeKey for Vector {
    type Denormalized = Vector;

    fn normalizations(&self) -> SmallVec<[Self; 2]> {
        smallvec![self.normalize().unwrap_or_else(|| self.clone())]
    }
    fn as_denormalized(&self) -> &Self::Denormalized {
        self
    }
}
impl KdTreeKeyDenormalized for Vector {
    fn origin() -> Self {
        Vector::EMPTY
    }
    fn offset_mut(&mut self, axis: u32, amount: f32) {
        *self += Vector::unit(axis as u8) * amount;
    }

    fn present_axes(&self) -> Set64<u32> {
        (0..self.ndim()).map(|i| i as u32).collect()
    }
    fn get_elem(&self, axis: u32) -> f32 {
        self.get(axis as u8)
    }
}

impl KdTreeKey for Rotor {
    type Denormalized = Multivector;

    fn normalizations(&self) -> SmallVec<[Self; 2]> {
        let normalized = self.clone().normalize().unwrap_or(self.clone());
        smallvec![normalized.clone(), -normalized]
    }
    fn as_denormalized(&self) -> &Self::Denormalized {
        self.multivector()
    }
}
impl KdTreeKeyDenormalized for Multivector {
    fn origin() -> Self {
        Multivector::ZERO
    }
    fn offset_mut(&mut self, axis: u32, amount: f32) {
        *self += Blade {
            coef: amount,
            axes: axis,
        }
    }

    fn present_axes(&self) -> Set64<u32> {
        self.blades().iter().map(|blade| blade.axes).collect()
    }
    fn get_elem(&self, axis: u32) -> f32 {
        self.get(axis).unwrap_or(0.0)
    }
}

impl KdTreeKey for Rotoreflector {
    type Denormalized = Multivector;

    fn normalizations(&self) -> SmallVec<[Self; 2]> {
        smallvec![self.clone(), -self]
    }
    fn as_denormalized(&self) -> &Self::Denormalized {
        self.multivector()
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;
    use std::collections::HashMap;
    use std::hash::{Hash, Hasher};

    use super::*;

    #[derive(Debug, Default, Copy, Clone)]
    struct SillyVector([Option<f32>; 5]);
    impl PartialEq for SillyVector {
        fn eq(&self, other: &Self) -> bool {
            self.canonicalize().0 == other.canonicalize().0
        }
    }
    impl Eq for SillyVector {}
    impl Hash for SillyVector {
        fn hash<H: Hasher>(&self, state: &mut H) {
            let canon = self.canonicalize();
            for i in 0..self.0.len() {
                canon.get(i).to_bits().hash(state);
            }
        }
    }
    impl SillyVector {
        fn get(self, i: usize) -> f32 {
            self.0.get(i).copied().flatten().unwrap_or(0.0)
        }
        fn canonicalize(self) -> Self {
            SillyVector(self.0.map(|x| x.filter(|&x| x != 0.0)))
        }
    }
    impl AbsDiffEq for SillyVector {
        type Epsilon = f32;

        fn default_epsilon() -> Self::Epsilon {
            EPSILON
        }

        fn abs_diff_eq(&self, other: &Self, epsilon: Self::Epsilon) -> bool {
            (0..self.0.len()).all(|i| self.get(i).abs_diff_eq(&other.get(i), epsilon))
        }
    }
    impl KdTreeKey for SillyVector {
        type Denormalized = SillyVector;

        fn normalizations(&self) -> SmallVec<[Self; 2]> {
            let mag = (0..self.0.len())
                .map(|i| self.get(i))
                .map(|x| x * x)
                .sum::<f32>()
                .sqrt();
            if mag == 0.0 {
                smallvec![*self]
            } else {
                smallvec![SillyVector(self.0.map(|x| Some(x? / mag)))]
            }
        }
        fn as_denormalized(&self) -> &Self::Denormalized {
            self
        }
    }
    impl KdTreeKeyDenormalized for SillyVector {
        fn origin() -> Self {
            SillyVector([None; 5])
        }

        fn offset_mut(&mut self, axis: u32, amount: f32) {
            self.0[axis as usize] = Some(self.get(axis as usize) + amount);
        }

        fn present_axes(&self) -> Set64<u32> {
            (0..self.0.len()).map(|i| i as u32).collect()
        }

        fn get_elem(&self, axis: u32) -> f32 {
            self.get(axis as usize)
        }
    }
    impl Arbitrary for SillyVector {
        type Parameters = ();

        fn arbitrary_with(_args: Self::Parameters) -> Self::Strategy {
            let elem_strategy = proptest::strategy::Union::new([
                Just(None),
                Just(Some(0.0)),
                Just(Some(1.0)),
                Just(Some(-1.0)),
                Just(Some(0.5)),
                Just(Some(-0.5)),
                Just(Some(1.5)),
                Just(Some(-1.5)),
            ]);
            proptest::array::uniform5(elem_strategy)
                .prop_map(SillyVector)
                .boxed()
        }

        type Strategy = BoxedStrategy<Self>;
    }

    fn silly_vector_list(
        range: impl Into<proptest::sample::SizeRange>,
    ) -> BoxedStrategy<Vec<SillyVector>> {
        proptest::collection::vec(SillyVector::arbitrary(), range).boxed()
    }

    proptest! {
        #[test]
        fn test_kdtree(
            data in silly_vector_list(0..10),
            test in silly_vector_list(0..10),
        ) {
            let mut hashmap = HashMap::new();
            let mut kdtree = KdTreeMap::new();


            // Put data in
            for (i, &v) in data.iter().enumerate() {
                assert_eq!(
                    hashmap.insert(v.canonicalize(), i),
                    kdtree.insert(&v, i),
                );
            }

            // Get the same data out
            for (v, i) in &hashmap {
                assert_eq!(Some(i), kdtree.get(v));
            }

            // Get different data out
            for &v in &test {
                assert_eq!(
                    hashmap.get(&v.canonicalize()),
                    kdtree.get(&v),
                );
            }

            // Nearest-neighbor query
            for v in &test {
                if data.is_empty() {
                    assert_eq!(None, kdtree.nearest(v));
                } else {
                    let min_distance = data
                        .iter()
                        .map(|u| v.normalizations()[0].distance_to(&u.normalizations()[0]))
                        .min_by(f32::total_cmp)
                        .unwrap();
                    let (nearest, _i) = kdtree.nearest(v).unwrap();
                    assert!(approx_eq(&min_distance, &v.normalizations()[0].distance_to(nearest)));
                }
            }

        }
    }
}
