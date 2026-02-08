use std::fmt;
use std::hash::Hash;
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
/// This is optimized to fit in exactly one `usize` and avoid allocation when
/// all layer numbers are sufficiently small (≤63 on 64-bit platforms, and ≤31
/// on 32-bit platforms). Higher layer numbers require allocation to store the
/// bitmask.
///
/// Bitwise operations are also incredibly efficient.
///
/// Be wary: constructing a `LayerMask` that contains a large layer number will
/// allocate bits for all layers smaller than it. For example,
/// `LayerMask::from_layer(Layer::new(300).unwrap())` allocates at least 40
/// bytes.
///
/// ## Other collections
///
/// For a sparse set of layers on axes with 64 or more layers, consider
/// [`hypuz_util::ti::TiSet`]`<`[`Layer`]`>`.
///
/// For a layer mask used in notation, see [`crate::LayerPrefix`].
pub union LayerMask {
    /// If `bitmask & 1 == 1`, then this is an inline bitmask. Otherwise it is
    /// a pointer to a [`BitVec`].
    inline_bitmask: usize,
    /// Pointer to a [`BitVec`].
    pointer: NonNull<BitVec>,
}

enum LayerMaskEnum<Ptr> {
    /// Inline bitmask, with the lowest bit equal to zero.
    Bitmask(usize),
    /// Pointer to a [`BitVec`].
    BitVec(Ptr),
}

impl<Ptr> LayerMaskEnum<Ptr> {
    fn map<U>(self, f: fn(Ptr) -> U) -> LayerMaskEnum<U> {
        match self {
            LayerMaskEnum::Bitmask(bits) => LayerMaskEnum::Bitmask(bits),
            LayerMaskEnum::BitVec(ptr) => LayerMaskEnum::BitVec(f(ptr)),
        }
    }
}

impl LayerMask {
    /// Empty layer set.
    pub const EMPTY: Self = Self::from_bits(0);

    fn as_usize_ref(&self) -> &usize {
        // SAFETY: it is always sound to reinterpret a `pointer` as a `usize`
        unsafe { &self.inline_bitmask }
    }
    fn as_usize(&self) -> usize {
        *self.as_usize_ref()
    }

    /// Returns whether this is an inline bitmask as opposed to a pointer.
    ///
    /// If this function returns `false`, then it is sound to access and
    /// to deference `pointer`.
    fn is_inline(&self) -> bool {
        self.as_usize() & 1 == 1
    }

    /// Returns whether this is a pointer, null or not.
    fn is_pointer(&self) -> bool {
        !self.is_inline()
    }

    /// Returns the inline bitmask, if this is not a pointer.
    fn as_inline(&self) -> Option<usize> {
        self.is_inline().then(|| self.as_usize() & !1)
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

    fn as_ptr_enum(&self) -> LayerMaskEnum<NonNull<BitVec>> {
        match self.as_pointer() {
            Some(ptr) => LayerMaskEnum::BitVec(ptr),
            None => LayerMaskEnum::Bitmask(self.as_inline().expect("expected bitmask")),
        }
    }
    fn as_ref_enum(&self) -> LayerMaskEnum<&BitVec> {
        // SAFETY: The pointer is always valid, and the returned lifetime is
        //         borrowing from `self`.
        self.as_ptr_enum().map(|ptr| unsafe { ptr.as_ref() })
    }
    fn as_mut_enum(&mut self) -> LayerMaskEnum<&mut BitVec> {
        // SAFETY: The pointer is always valid, and the returned lifetime is
        //         borrowing mutably from `self`.
        self.as_ptr_enum().map(|mut ptr| unsafe { ptr.as_mut() })
    }

    /// Returns the first `usize` bits of the bitmask. The lowest bit is always
    /// zero.
    fn first_usize(&self) -> usize {
        match self.as_ref_enum() {
            LayerMaskEnum::Bitmask(bits) => bits,
            LayerMaskEnum::BitVec(vec) => *vec.as_raw_slice().first().unwrap_or(&0),
        }
    }

    /// Constructs a layer set from a bitmask, ignoring the least significant
    /// bit.
    const fn from_bits(bits: usize) -> Self {
        // SAFETY: bit-or with `1` to indicate that this is a bitmask.
        let inline_bitmask = bits | 1;
        Self { inline_bitmask }
    }
    fn from_bitvec(mut vec: Box<BitVec>) -> Self {
        assert_ptr_tagging_exists();
        vec.set(0, false);
        let pointer = NonNull::new(Box::into_raw(vec)).expect("Box pointer must be non-null");
        Self { pointer }
    }

    /// Returns whether the set is empty.
    pub fn is_empty(&self) -> bool {
        match self.as_ref_enum() {
            LayerMaskEnum::Bitmask(bits) => bits == 0,
            LayerMaskEnum::BitVec(vec) => vec.as_raw_slice().iter().all(|&bits| bits == 0),
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
            LayerMaskEnum::Bitmask(bits) => {
                *self = Self::from_bits(bits | (1 << l));
            }
            LayerMaskEnum::BitVec(vec) => {
                vec.set(l, true);
            }
        }
    }

    /// Adds a layer range to the set.
    pub fn insert_range(&mut self, range: LayerRange) {
        self.set_min_capacity(range.end());
        match self.as_mut_enum() {
            LayerMaskEnum::Bitmask(bits) => *self = Self::from_bits(bits | range_mask_usize(range)),
            LayerMaskEnum::BitVec(vec) => {
                vec[range.start().to_usize()..=range.end().to_usize()].fill(true);
            }
        }
    }

    /// Removes a layer from the set.
    pub fn remove(&mut self, layer: Layer) {
        let l = layer.to_usize();
        match self.as_mut_enum() {
            LayerMaskEnum::Bitmask(bits) => {
                if l < usize::BITS as usize {
                    *self = Self::from_bits(bits & !(1 << l));
                }
            }
            LayerMaskEnum::BitVec(vec) => {
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
            LayerMaskEnum::Bitmask(bits) => {
                *self = Self::from_bits(bits & !range_mask_usize(range));
            }
            LayerMaskEnum::BitVec(vec) => {
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
            LayerMaskEnum::Bitmask(_) => usize::BITS as u16 - 1,
            LayerMaskEnum::BitVec(vec) => (vec.len() - 1) as u16,
        })
    }

    /// Grows the set if necessary to ensure that `max_layer` fits.
    fn set_min_capacity(&mut self, max_layer: Layer) {
        if max_layer >= usize::BITS
            && let Some(bitmask) = self.as_inline()
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
    ///
    /// This may remove layers from the mask.
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
            LayerMaskEnum::Bitmask(bits) => {
                if bits == 0 {
                    *self = Self::EMPTY;
                }
            }
            LayerMaskEnum::BitVec(vec) => match vec.last_one() {
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
    ///
    /// The returned bitslice may be empty.
    fn as_bitslice_from_1(&self) -> &BitSlice {
        match self.as_ref_enum() {
            LayerMaskEnum::Bitmask(_) => &BitSlice::from_element(self.as_usize_ref())[1..],
            LayerMaskEnum::BitVec(vec) => &vec[1..],
        }
    }

    /// Returns a bit slice **excluding bit 0** and truncated as much as
    /// possible without removing any layers present in the mask.
    ///
    /// The returned bitslice may be empty.
    fn as_truncated_bitslice_from_1(&self) -> &BitSlice {
        let bitslice = self.as_bitslice_from_1();
        match bitslice.last_one() {
            Some(i) => &bitslice[..i + 1],
            None => BitSlice::empty(),
        }
    }

    /// Iterates in order over layers in the set.
    pub fn iter(&self) -> LayerSetIter<'_> {
        LayerSetIter(self.as_bitslice_from_1().iter_ones())
    }
}

impl Drop for LayerMask {
    fn drop(&mut self) {
        if let LayerMaskEnum::BitVec(ptr) = self.as_ptr_enum() {
            // SAFETY: The pointer is valid. The value is veing dropped so it is
            //         safe to take ownership.
            drop(unsafe { Box::from_raw(ptr.as_ptr()) });
        }
    }
}

impl fmt::Debug for LayerMask {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut first = true;
        write!(f, "LayerMask {{")?;
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

impl fmt::Display for LayerMask {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", crate::LayerSet::from(self))
    }
}

impl Clone for LayerMask {
    fn clone(&self) -> Self {
        match self.as_ref_enum() {
            LayerMaskEnum::Bitmask(bits) => Self::from_bits(bits),
            LayerMaskEnum::BitVec(vec) => Self::from_bitvec(Box::new(vec.clone())),
        }
    }
}

impl Default for LayerMask {
    fn default() -> Self {
        Self::new()
    }
}

impl FromIterator<Layer> for LayerMask {
    fn from_iter<T: IntoIterator<Item = Layer>>(iter: T) -> Self {
        let mut ret = Self::new();
        for layer in iter {
            ret.insert(layer);
        }
        ret
    }
}

impl FromIterator<LayerRange> for LayerMask {
    fn from_iter<T: IntoIterator<Item = LayerRange>>(iter: T) -> Self {
        let mut ret = Self::new();
        for range in iter {
            ret.insert_range(range);
        }
        ret
    }
}

impl<'a> IntoIterator for &'a LayerMask {
    type Item = Layer;

    type IntoIter = LayerSetIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl PartialEq for LayerMask {
    fn eq(&self, other: &Self) -> bool {
        self.as_truncated_bitslice_from_1() == other.as_truncated_bitslice_from_1()
    }
}

impl Eq for LayerMask {}

impl PartialOrd for LayerMask {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for LayerMask {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.as_truncated_bitslice_from_1()
            .cmp(other.as_truncated_bitslice_from_1())
    }
}

impl Hash for LayerMask {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.as_truncated_bitslice_from_1().hash(state);
    }
}

impl BitAnd for &LayerMask {
    type Output = LayerMask;

    fn bitand(self, rhs: Self) -> Self::Output {
        let max_layer = std::cmp::min(self.capacity(), rhs.capacity());
        self.clone_resized(max_layer) & rhs
    }
}

impl BitAnd<&LayerMask> for LayerMask {
    type Output = LayerMask;

    fn bitand(mut self, rhs: &LayerMask) -> Self::Output {
        self &= rhs;
        self
    }
}

impl BitAnd for LayerMask {
    type Output = LayerMask;

    fn bitand(mut self, rhs: Self) -> Self::Output {
        self &= rhs;
        self
    }
}

impl BitAndAssign<&LayerMask> for LayerMask {
    fn bitand_assign(&mut self, rhs: &LayerMask) {
        let len = std::cmp::min(self.capacity(), rhs.capacity()).to_usize();
        match self.as_mut_enum() {
            LayerMaskEnum::Bitmask(b1) => {
                let b2 = rhs.first_usize();
                *self = Self::from_bits(b1 & b2);
            }
            LayerMaskEnum::BitVec(b1) => {
                let b2 = rhs.as_bitslice_from_1();
                b1[1..=len] &= &b2[..len];
                b1[len + 1..].fill(false);
            }
        }
    }
}

impl BitAndAssign for LayerMask {
    fn bitand_assign(&mut self, rhs: Self) {
        *self &= &rhs;
    }
}

impl BitOr for &LayerMask {
    type Output = LayerMask;

    fn bitor(self, rhs: Self) -> Self::Output {
        let max_layer = std::cmp::max(self.capacity(), rhs.capacity());
        self.clone_resized(max_layer) | rhs
    }
}

impl BitOr<&LayerMask> for LayerMask {
    type Output = LayerMask;

    fn bitor(mut self, rhs: &LayerMask) -> Self::Output {
        self |= rhs;
        self
    }
}

impl BitOr for LayerMask {
    type Output = LayerMask;

    fn bitor(mut self, rhs: Self) -> Self::Output {
        self |= rhs;
        self
    }
}

impl BitOrAssign<&LayerMask> for LayerMask {
    fn bitor_assign(&mut self, rhs: &LayerMask) {
        self.set_min_capacity(rhs.capacity());
        match self.as_mut_enum() {
            LayerMaskEnum::Bitmask(b1) => {
                let b2 = rhs.first_usize();
                *self = Self::from_bits(b1 | b2);
            }
            LayerMaskEnum::BitVec(b1) => {
                let b2 = rhs.as_bitslice_from_1();
                b1[1..=b2.len()] |= b2;
            }
        }
    }
}

impl BitOrAssign for LayerMask {
    fn bitor_assign(&mut self, rhs: Self) {
        *self |= &rhs;
    }
}

impl BitXor for &LayerMask {
    type Output = LayerMask;

    fn bitxor(self, rhs: Self) -> Self::Output {
        let max_layer = std::cmp::max(self.capacity(), rhs.capacity());
        self.clone_resized(max_layer) ^ rhs
    }
}

impl BitXor<&LayerMask> for LayerMask {
    type Output = LayerMask;

    fn bitxor(mut self, rhs: &LayerMask) -> Self::Output {
        self ^= rhs;
        self
    }
}

impl BitXor for LayerMask {
    type Output = LayerMask;

    fn bitxor(mut self, rhs: Self) -> Self::Output {
        self ^= rhs;
        self
    }
}

impl BitXorAssign<&LayerMask> for LayerMask {
    fn bitxor_assign(&mut self, rhs: &LayerMask) {
        self.set_min_capacity(rhs.capacity());
        match self.as_mut_enum() {
            LayerMaskEnum::Bitmask(b1) => {
                let b2 = rhs.first_usize();
                *self = Self::from_bits(b1 ^ b2);
            }
            LayerMaskEnum::BitVec(b1) => {
                let b2 = rhs.as_bitslice_from_1();
                b1[1..=b2.len()] ^= b2;
            }
        }
    }
}

impl BitXorAssign for LayerMask {
    fn bitxor_assign(&mut self, rhs: Self) {
        *self ^= &rhs;
    }
}

/// Iterator over layers in a [`LayerMask`].
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

#[cfg(feature = "serde")]
impl serde::Serialize for LayerMask {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use hypuz_util::serde_impl::hex_bitvec;

        match self.as_ref_enum() {
            LayerMaskEnum::Bitmask(bits) => {
                hex_bitvec::serialize(&BitVec::from_element(bits), serializer)
            }
            LayerMaskEnum::BitVec(vec) => hex_bitvec::serialize(vec, serializer),
        }
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for LayerMask {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let bitvec = hypuz_util::serde_impl::hex_bitvec::deserialize(deserializer)?;
        let mut ret = Self::from_bitvec(Box::new(bitvec));
        ret.shrink_to_fit();
        Ok(ret)
    }
}

#[cfg(test)]
impl proptest::arbitrary::Arbitrary for LayerMask {
    type Parameters = ();

    fn arbitrary_with((): Self::Parameters) -> Self::Strategy {
        use proptest::prelude::*;

        prop_oneof![
            usize::arbitrary().prop_map(|bits| LayerMask::from_bits(bits)),
            <[usize; 3]>::arbitrary()
                .prop_map(|elems| LayerMask::from_bitvec(Box::new(BitVec::from_slice(&elems))))
        ]
        .boxed()
    }

    type Strategy = proptest::strategy::BoxedStrategy<Self>;
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use proptest::prelude::*;

    use super::*;

    proptest! {
        #[test]
        fn proptest_layer_mask_insert_remove(ops in prop::collection::vec(arbitrary_op(), 0..10)) {
            let mut expected = HashSet::new();
            let mut actual = LayerMask::new();
            for op in ops {
                op.apply_to_hash_set(&mut expected);
                op.apply_to_layer_set(&mut actual);
            }

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

    fn assert_consistency(expected: &HashSet<Layer>, actual: &LayerMask) {
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
            Just(Op::ShrinkToFit),
        ]
    }

    #[derive(Debug, Copy, Clone)]
    enum Op {
        Insert(Layer),
        Remove(Layer),
        InsertRange(LayerRange),
        RemoveRange(LayerRange),
        ShrinkToFit,
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
                Op::ShrinkToFit => (),
            }
        }

        fn apply_to_layer_set(self, set: &mut LayerMask) {
            match self {
                Op::Insert(layer) => set.insert(layer),
                Op::Remove(layer) => set.remove(layer),
                Op::InsertRange(range) => set.insert_range(range),
                Op::RemoveRange(range) => set.remove_range(range),
                Op::ShrinkToFit => set.shrink_to_fit(),
            }
        }
    }

    proptest! {
        #[test]
        fn proptest_layer_mask_bit_ops(l1: LayerMask, l2: LayerMask) {
            let s1: HashSet<Layer> = l1.iter().collect();
            let s2: HashSet<Layer> = l2.iter().collect();
            let intersection: HashSet<Layer> = s1.intersection(&s2).copied().collect();
            let union: HashSet<Layer> = s1.union(&s2).copied().collect();
            let symmetric_difference: HashSet<Layer> = s1.symmetric_difference(&s2).copied().collect();
            assert_eq!(intersection, (&l1 & &l2).iter().collect());
            assert_eq!(union, (&l1 | &l2).iter().collect());
            assert_eq!(symmetric_difference, (&l1 ^ &l2).iter().collect());
        }

        #[test]
        fn proptest_layer_mask_from_iterator(l1: LayerMask) {
            let l2 = LayerMask::from_iter(&l1);
            let s1: HashSet<Layer> = l1.iter().collect();
            let s2: HashSet<Layer> = l2.iter().collect();
            assert_eq!(s1, s2);
            assert_eq!(l1, l2);
        }

        #[test]
        fn proptest_layer_mask_eq(l1: LayerMask, l2: LayerMask) {
            let s1: HashSet<Layer> = l1.iter().collect();
            let s2: HashSet<Layer> = l2.iter().collect();
            assert_eq!(s1 == s2, l1 == l2);
        }
    }
}
