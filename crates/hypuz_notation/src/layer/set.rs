use std::fmt;
use std::iter::FusedIterator;
use std::ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign, BitXor, BitXorAssign};
use std::ptr::NonNull;

use bitvec::order::Lsb0;
use bitvec::slice::{BitSlice, IterOnes};
use bitvec::vec::BitVec;

use crate::{Layer, LayerRange};

fn assert_ptr_tagging_exists() {
    // See https://github.com/rust-lang/rfcs/issues/1748
    assert!(
        std::mem::align_of::<BitVec>() >= 2,
        "target must support pointer tagging",
    );
}

/// Bitmask of positive [`Layer`]s.
///
/// This is optimized to fit in only one `usize` and avoid allocation when all
/// layer numbers are sufficiently small (≤63 on 64-bit platforms, and ≤31 on
/// 32-bit platforms) but higher layer numbers require allocation to store the
/// bitmask.
///
/// Be wary: constructing a `LayerSet` that contains a large layer number will
/// allocate bits for all layers smaller than it. For example,
/// `LayerSet::from_layer(Layer::new(300).unwrap())` allocates at least 40
/// bytes.
pub union LayerSet {
    /// If `bits & 1 == 1`, then this is a bitset. Otherwise it is
    /// a pointer.
    bitmask: usize,
    /// Pointer to a bitset.
    ///
    /// `usize` is always
    pointer: NonNull<BitVec>,
}

enum LayerSetEnum<Ptr> {
    /// Bitmask. The lowest bit is always zero.
    Bitmask(usize),
    BitVec(Ptr),
}

impl<Ptr> LayerSetEnum<Ptr> {
    fn map<U>(self, f: fn(Ptr) -> U) -> LayerSetEnum<U> {
        match self {
            LayerSetEnum::Bitmask(bits) => LayerSetEnum::Bitmask(bits),
            LayerSetEnum::BitVec(ptr) => LayerSetEnum::BitVec(f(ptr)),
        }
    }
}

impl LayerSet {
    /// Empty layer set.
    pub const EMPTY: Self = Self::from_bits(0);

    fn as_usize_ref(&self) -> &usize {
        // SAFETY: it is always sound to reinterpret a `pointer` as a `usize`
        unsafe { &self.bitmask }
    }
    fn as_usize(&self) -> usize {
        *self.as_usize_ref()
    }

    /// Returns whether this is a bitset as opposed to a pointer.
    ///
    /// If this function returns `false`, then it is sound to access and
    /// to deference `pointer`.
    fn is_bitset(&self) -> bool {
        self.as_usize() & 1 == 1
    }

    /// Returns whether this is a pointer, null or not.
    fn is_pointer(&self) -> bool {
        !self.is_bitset()
    }

    /// Returns the bitmask, if this is not a pointer.
    fn as_bitmask(&self) -> Option<usize> {
        self.is_bitset().then(|| self.as_usize() & !1)
    }

    /// Returns the pointer, if this is a pointer.
    fn as_pointer(&self) -> Option<NonNull<BitVec>> {
        // accessing `self.pointer` when it's not valid is instant UB
        #[allow(clippy::unnecessary_lazy_evaluations)]
        // SAFETY: `is_pointer()` returned `true`, so we can access
        //         `self.pointer`.
        self.is_pointer().then(|| unsafe { self.pointer })
    }
    /// Returns a mutable reference to the `BitVec`, if this is a pointer.
    fn as_mut_bitvec(&mut self) -> Option<&mut BitVec> {
        // SAFETY: The pointer is always valid, and the returned lifetime is
        //         borrowing from `self`.
        self.as_pointer().map(|mut ptr| unsafe { ptr.as_mut() })
    }

    fn as_ptr_enum(&self) -> LayerSetEnum<NonNull<BitVec>> {
        match self.as_pointer() {
            Some(ptr) => LayerSetEnum::BitVec(ptr),
            None => LayerSetEnum::Bitmask(self.as_bitmask().expect("expected bitmask")),
        }
    }
    fn as_ref_enum(&self) -> LayerSetEnum<&BitVec> {
        // SAFETY: The pointer is always valid, and the returned lifetime is
        //         borrowing from `self`.
        self.as_ptr_enum().map(|ptr| unsafe { ptr.as_ref() })
    }
    fn as_mut_enum(&mut self) -> LayerSetEnum<&mut BitVec> {
        // SAFETY: The pointer is always valid, and the returned lifetime is
        //         borrowing mutably from `self`.
        self.as_ptr_enum().map(|mut ptr| unsafe { ptr.as_mut() })
    }

    /// Returns the first `usize` bits of the bitmask. The lowest bit is always
    /// zero.
    fn first_usize(&self) -> usize {
        match self.as_ref_enum() {
            LayerSetEnum::Bitmask(bits) => bits,
            LayerSetEnum::BitVec(vec) => *vec.as_raw_slice().first().unwrap_or(&0),
        }
    }

    /// Constructs a layer set from a bitmask, ignoring the least significant
    /// bit.
    const fn from_bits(bits: usize) -> Self {
        // SAFETY: bit-or with `1` to indicate that this is a bitset.
        Self { bitmask: bits | 1 }
    }
    fn from_bitvec(vec: Box<BitVec>) -> Self {
        assert_ptr_tagging_exists();
        let pointer = NonNull::new(Box::into_raw(vec)).expect("Box pointer must be non-null");
        Self { pointer }
    }

    /// Returns whether the set is empty.
    pub fn is_empty(&self) -> bool {
        match self.as_ref_enum() {
            LayerSetEnum::Bitmask(bits) => bits == 0,
            LayerSetEnum::BitVec(vec) => vec.as_raw_slice().iter().all(|&bits| bits == 0),
        }
    }

    /// Constructs an empty set.
    pub fn new() -> Self {
        Self::EMPTY
    }

    /// Constructs a set containing a single layer.
    pub fn from_layer(layer: Layer) -> Self {
        let mut ret = Self::new();
        ret.insert(layer);
        ret
    }

    /// Constructs a set containing all layers in a range.
    pub fn from_range(range: LayerRange) -> Self {
        let mut ret = Self::new();
        ret.insert_range(range);
        ret
    }

    /// Adds a layer to the set.
    pub fn insert(&mut self, layer: Layer) {
        let l = layer.to_usize();
        self.set_min_capacity(layer);
        match self.as_mut_enum() {
            LayerSetEnum::Bitmask(bits) => {
                *self = Self::from_bits(bits | (1 << l));
            }
            LayerSetEnum::BitVec(vec) => {
                vec.set(l, true);
            }
        }
    }

    /// Adds a layer range to the set.
    pub fn insert_range(&mut self, range: LayerRange) {
        self.set_min_capacity(range.end());
        match self.as_mut_enum() {
            LayerSetEnum::Bitmask(bits) => *self = Self::from_bits(bits | range_mask_usize(range)),
            LayerSetEnum::BitVec(vec) => {
                vec[range.start().to_usize()..=range.end().to_usize()].fill(true);
            }
        }
    }

    /// Removes a layer from the set.
    pub fn remove(&mut self, layer: Layer) {
        let l = layer.to_usize();
        match self.as_mut_enum() {
            LayerSetEnum::Bitmask(bits) => {
                if l < usize::BITS as usize {
                    *self = Self::from_bits(bits & !(1 << l));
                }
            }
            LayerSetEnum::BitVec(vec) => {
                if l < vec.len() {
                    vec.set(l, false);
                }
            }
        }
    }

    /// Removes a layer range from the set.
    pub fn remove_range(&mut self, range: LayerRange) {
        let Some(range) = range.clamp_to_layer_count(self.capacity().to_u16()) else {
            return;
        };
        match self.as_mut_enum() {
            LayerSetEnum::Bitmask(bits) => *self = Self::from_bits(bits & !range_mask_usize(range)),
            LayerSetEnum::BitVec(vec) => {
                vec[range.start().to_usize()..=range.end().to_usize()].fill(false);
            }
        }
    }

    /// Removes all layers from the layer set.
    pub fn clear(&mut self) {
        *self = Self::EMPTY;
    }

    /// Returns whether `layer` is in the set.
    pub fn contains(&self, layer: Layer) -> bool {
        match self.as_bitslice_from_1().get(layer.to_usize() - 1) {
            Some(bitref) => *bitref,
            None => false,
        }
    }

    /// Returns the maximum layer that fits in the set at its current size.
    fn capacity(&self) -> Layer {
        Layer::new_clamped(match self.as_ref_enum() {
            LayerSetEnum::Bitmask(_) => usize::BITS as u16 - 1,
            LayerSetEnum::BitVec(vec) => (vec.len() - 1) as u16,
        })
    }

    /// Grows the set if necessary to ensure that `max_layer` fits.
    fn set_min_capacity(&mut self, max_layer: Layer) {
        if max_layer >= usize::BITS
            && let Some(bitmask) = self.as_bitmask()
        {
            *self = Self::from_bitvec(Box::new(BitVec::from_element(bitmask)));
        }
        if let Some(bitvec) = self.as_mut_bitvec()
            && bitvec.len() <= max_layer.to_usize()
        {
            bitvec.resize(max_layer.to_usize() + 1, false);
        }
    }

    /// Clones the set and resizes it to fit exactly `new_capacity`.
    fn clone_resized(&self, max_layer: Layer) -> Self {
        if max_layer >= usize::BITS {
            let mut vec = BitVec::repeat(false, max_layer.to_usize() + 1);
            let bits = self.as_bitslice_from_1();
            vec[1..=bits.len()].copy_from_bitslice(bits);
            Self::from_bitvec(Box::new(vec))
        } else {
            Self::from_bits(self.first_usize())
        }
    }

    /// Shrinks the capacity of the allocation as much as possible or
    /// deallocates if possible, if there is an allocation.
    ///
    /// This canonicalizes the layer set.
    pub fn shrink_to_fit(&mut self) {
        match self.as_mut_enum() {
            LayerSetEnum::Bitmask(bits) => {
                if bits == 0 {
                    *self = Self::EMPTY;
                }
            }
            LayerSetEnum::BitVec(vec) => match vec.last_one() {
                None => *self = Self::EMPTY,
                Some(i) => {
                    if i < usize::BITS as usize {
                        let bits = *vec.as_raw_slice().first().unwrap_or(&0);
                        *self = Self::from_bits(bits);
                    } else {
                        vec.truncate(i + 1);
                    }
                }
            },
        }
    }

    /// Returns a bit slice **excluding bit 0**.
    fn as_bitslice_from_1(&self) -> &BitSlice {
        match self.as_ref_enum() {
            LayerSetEnum::Bitmask(_) => &BitSlice::from_element(self.as_usize_ref())[1..],
            LayerSetEnum::BitVec(vec) => &vec[1..],
        }
    }

    /// Iterates in order over layers in the set.
    pub fn iter(&self) -> LayerSetIter<'_> {
        LayerSetIter(self.as_bitslice_from_1().iter_ones())
    }
}

impl Drop for LayerSet {
    fn drop(&mut self) {
        if let LayerSetEnum::BitVec(ptr) = self.as_ptr_enum() {
            // SAFETY: The pointer is valid. The value is veing dropped so it is
            //         safe to take ownership.
            drop(unsafe { Box::from_raw(ptr.as_ptr()) });
        }
    }
}

impl fmt::Debug for LayerSet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut first = true;
        write!(f, "LayerSet {{")?;
        for l in self.iter() {
            if first {
                first = false;
            } else {
                write!(f, ",")?;
            }
            write!(f, " {l}")?;
        }
        write!(f, " }}")?;
        Ok(())
    }
}

impl fmt::Display for LayerSet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", crate::LayerMask::from(self))
    }
}

impl Clone for LayerSet {
    fn clone(&self) -> Self {
        match self.as_ref_enum() {
            LayerSetEnum::Bitmask(bits) => Self::from_bits(bits),
            LayerSetEnum::BitVec(vec) => Self::from_bitvec(Box::new(vec.clone())),
        }
    }
}

impl Default for LayerSet {
    fn default() -> Self {
        Self::new()
    }
}

impl FromIterator<Layer> for LayerSet {
    fn from_iter<T: IntoIterator<Item = Layer>>(iter: T) -> Self {
        let mut ret = Self::new();
        for layer in iter {
            ret.insert(layer);
        }
        ret
    }
}

impl FromIterator<LayerRange> for LayerSet {
    fn from_iter<T: IntoIterator<Item = LayerRange>>(iter: T) -> Self {
        let mut ret = Self::new();
        for range in iter {
            ret.insert_range(range);
        }
        ret
    }
}

impl<'a> IntoIterator for &'a LayerSet {
    type Item = Layer;

    type IntoIter = LayerSetIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl BitAnd for &LayerSet {
    type Output = LayerSet;

    fn bitand(self, rhs: Self) -> Self::Output {
        let max_layer = std::cmp::min(self.capacity(), rhs.capacity());
        self.clone_resized(max_layer) & rhs
    }
}

impl BitAnd<&LayerSet> for LayerSet {
    type Output = LayerSet;

    fn bitand(mut self, rhs: &LayerSet) -> Self::Output {
        self &= rhs;
        self
    }
}

impl BitAnd for LayerSet {
    type Output = LayerSet;

    fn bitand(mut self, rhs: Self) -> Self::Output {
        self &= rhs;
        self
    }
}

impl BitAndAssign<&LayerSet> for LayerSet {
    fn bitand_assign(&mut self, rhs: &LayerSet) {
        let len = std::cmp::min(self.capacity(), rhs.capacity()).to_usize();
        match self.as_mut_enum() {
            LayerSetEnum::Bitmask(b1) => {
                let b2 = rhs.first_usize();
                *self = Self::from_bits(b1 & b2);
            }
            LayerSetEnum::BitVec(b1) => {
                let b2 = rhs.as_bitslice_from_1();
                b1[1..=len] &= &b2[..len];
                b1[len + 1..].fill(false);
            }
        }
    }
}

impl BitAndAssign for LayerSet {
    fn bitand_assign(&mut self, rhs: Self) {
        *self &= &rhs;
    }
}

impl BitOr for &LayerSet {
    type Output = LayerSet;

    fn bitor(self, rhs: Self) -> Self::Output {
        let max_layer = std::cmp::max(self.capacity(), rhs.capacity());
        self.clone_resized(max_layer) | rhs
    }
}

impl BitOr<&LayerSet> for LayerSet {
    type Output = LayerSet;

    fn bitor(mut self, rhs: &LayerSet) -> Self::Output {
        self |= rhs;
        self
    }
}

impl BitOr for LayerSet {
    type Output = LayerSet;

    fn bitor(mut self, rhs: Self) -> Self::Output {
        self |= rhs;
        self
    }
}

impl BitOrAssign<&LayerSet> for LayerSet {
    fn bitor_assign(&mut self, rhs: &LayerSet) {
        self.set_min_capacity(rhs.capacity());
        match self.as_mut_enum() {
            LayerSetEnum::Bitmask(b1) => {
                let b2 = rhs.first_usize();
                *self = Self::from_bits(b1 | b2);
            }
            LayerSetEnum::BitVec(b1) => {
                let b2 = rhs.as_bitslice_from_1();
                b1[1..=b2.len()] |= b2;
            }
        }
    }
}

impl BitOrAssign for LayerSet {
    fn bitor_assign(&mut self, rhs: Self) {
        *self |= &rhs;
    }
}

impl BitXor for &LayerSet {
    type Output = LayerSet;

    fn bitxor(self, rhs: Self) -> Self::Output {
        let max_layer = std::cmp::max(self.capacity(), rhs.capacity());
        self.clone_resized(max_layer) ^ rhs
    }
}

impl BitXor<&LayerSet> for LayerSet {
    type Output = LayerSet;

    fn bitxor(mut self, rhs: &LayerSet) -> Self::Output {
        self ^= rhs;
        self
    }
}

impl BitXor for LayerSet {
    type Output = LayerSet;

    fn bitxor(mut self, rhs: Self) -> Self::Output {
        self ^= rhs;
        self
    }
}

impl BitXorAssign<&LayerSet> for LayerSet {
    fn bitxor_assign(&mut self, rhs: &LayerSet) {
        self.set_min_capacity(rhs.capacity());
        match self.as_mut_enum() {
            LayerSetEnum::Bitmask(b1) => {
                let b2 = rhs.first_usize();
                *self = Self::from_bits(b1 ^ b2);
            }
            LayerSetEnum::BitVec(b1) => {
                let b2 = rhs.as_bitslice_from_1();
                b1[1..=b2.len()] ^= b2;
            }
        }
    }
}

impl BitXorAssign for LayerSet {
    fn bitxor_assign(&mut self, rhs: Self) {
        *self ^= &rhs;
    }
}

/// Iterator over layers in a [`LayerSet`].
///
/// Layers are yielded in ascending order (shallowest to deepest) with no
/// duplicates.
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq)]
pub struct LayerSetIter<'a>(IterOnes<'a, usize, Lsb0>);

impl Iterator for LayerSetIter<'_> {
    type Item = Layer;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(panicking_layer_from_index)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.0.size_hint()
    }

    fn count(self) -> usize {
        self.0.count()
    }

    fn last(self) -> Option<Self::Item> {
        self.0.last().map(panicking_layer_from_index)
    }
}

impl DoubleEndedIterator for LayerSetIter<'_> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.0.next_back().map(panicking_layer_from_index)
    }
}

impl ExactSizeIterator for LayerSetIter<'_> {}

impl FusedIterator for LayerSetIter<'_> {}

fn range_mask_usize(range: LayerRange) -> usize {
    assert!(range.end() < usize::MAX);
    1_usize
        .unbounded_shl(range.end().to_u16() as u32 + 1)
        .wrapping_sub(1)
        & !1_usize
            .unbounded_shl(range.start().to_u16() as u32)
            .wrapping_sub(1)
}

fn panicking_layer_from_index(n: usize) -> Layer {
    Layer::new((n + 1) as u16).expect("layer out of range")
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use proptest::prelude::*;

    use super::*;

    proptest! {
        #[test]
        fn proptest_layer_set_ops(ops in prop::collection::vec(arbitrary_op(), 0..10)) {
            let mut expected = HashSet::new();
            let mut actual = LayerSet::new();
            for op in ops {
                op.apply_to_hash_set(&mut expected);
                op.apply_to_layer_set(&mut actual);
            }

            assert_consistency(&expected, &actual);
            actual.shrink_to_fit();
            assert_consistency(&expected, &actual);
        }
    }

    fn small_layer() -> impl Strategy<Value = Layer> {
        (1..=200_u16).prop_map(|i| Layer::new(i).unwrap())
    }

    fn small_layer_range() -> impl Strategy<Value = LayerRange> {
        (1..=150_u16, 0..25_u16).prop_map(|(start, len)| {
            LayerRange::new(Layer::new(start).unwrap(), Layer::new(start + len).unwrap())
        })
    }

    fn assert_consistency(expected: &HashSet<Layer>, actual: &LayerSet) {
        assert_eq!(*expected, actual.iter().collect());
        for l in (1..=200).map(|i| Layer::new(i).unwrap()) {
            assert_eq!(expected.contains(&l), actual.contains(l))
        }
    }

    fn arbitrary_op() -> impl Strategy<Value = Op> {
        prop_oneof![
            small_layer().prop_map(Op::Insert),
            small_layer().prop_map(Op::Remove),
            small_layer_range().prop_map(Op::InsertRange),
            small_layer_range().prop_map(Op::RemoveRange),
        ]
    }

    #[derive(Debug, Copy, Clone)]
    enum Op {
        Insert(Layer),
        Remove(Layer),
        InsertRange(LayerRange),
        RemoveRange(LayerRange),
    }

    impl Op {
        fn apply_to_hash_set(self, set: &mut HashSet<Layer>) {
            match self {
                Op::Insert(layer) => {
                    set.insert(layer);
                }
                Op::Remove(layer) => {
                    set.remove(&layer);
                }
                Op::InsertRange(range) => {
                    for l in range.into_iter() {
                        set.insert(l);
                    }
                }
                Op::RemoveRange(range) => {
                    for l in range.into_iter() {
                        set.remove(&l);
                    }
                }
            }
        }

        fn apply_to_layer_set(self, set: &mut LayerSet) {
            match self {
                Op::Insert(layer) => set.insert(layer),
                Op::Remove(layer) => set.remove(layer),
                Op::InsertRange(range) => set.insert_range(range),
                Op::RemoveRange(range) => set.remove_range(range),
            }
        }
    }
}
