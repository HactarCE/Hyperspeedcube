use std::marker::PhantomData;
use std::ops::{BitAndAssign, BitOrAssign, BitXorAssign, Not};

use bitvec::bitbox;
use bitvec::boxed::BitBox;
use bitvec::vec::BitVec;

use super::IndexNewtype;

/// Dense subset of elements from a `GenericVec`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct GenericMask<I> {
    /// Bitmask of elements.
    ///
    /// Uninitialized bits must be set to zero!
    bits: BitBox,
    _phantom: PhantomData<I>,
}

impl<I> GenericMask<I> {
    /// Constructs an empty set.
    pub fn new_empty(len: usize) -> Self {
        Self {
            bits: bitbox![0; len],
            _phantom: PhantomData,
        }
        .clear_uninit()
    }
    /// Constructs a set containing every element in the puzzle.
    pub fn new_full(len: usize) -> Self {
        Self {
            bits: bitbox![1; len],
            _phantom: PhantomData,
        }
        .clear_uninit()
    }
    /// Clears uninitialized bits.
    #[must_use]
    fn clear_uninit(mut self) -> Self {
        self.bits.fill_uninitialized(false);
        self
    }

    /// Flips every bit.
    #[must_use]
    pub fn complement(mut self) -> Self {
        self.bits = !self.bits;
        self
    }

    /// Returns the total number of elements in the puzzle.
    pub fn max_len(&self) -> usize {
        self.bits.len()
    }
    /// Returns the number of pieces in the set.
    pub fn len(&self) -> usize {
        self.bits.count_ones()
    }
    /// Returns whether the set is empty.
    pub fn is_empty(&self) -> bool {
        self.bits.not_any()
    }

    fn raw_iter(&self) -> impl '_ + Iterator<Item = usize> {
        self.bits.as_raw_slice().iter().copied()
    }
    fn raw_iter_mut(&mut self) -> impl Iterator<Item = &mut usize> {
        self.bits.as_raw_mut_slice().iter_mut()
    }
    fn from_raw_iter(len: usize, raw_values: impl Iterator<Item = usize>) -> Self {
        let mut vec = BitVec::from_vec(raw_values.collect());
        vec.truncate(len);
        assert_eq!(vec.len(), len, "not enough values for bitbox");
        Self {
            bits: vec.into_boxed_bitslice(),
            _phantom: PhantomData,
        }
        .clear_uninit()
    }

    /// Returns whether `self` and `other` have nonempty intersection.
    pub fn any_overlap(&self, other: &Self) -> bool {
        std::iter::zip(self.raw_iter(), other.raw_iter()).any(|(l, r)| l & r != 0)
    }
}

impl<I: IndexNewtype> GenericMask<I> {
    /// Constructs a set containing a single element.
    pub fn from_element(len: usize, element: I) -> Self {
        Self::from_iter(len, [element])
    }
    /// Constructs a set containing every element from `iter`.
    pub fn from_iter(len: usize, iter: impl IntoIterator<Item = I>) -> Self {
        let mut ret = Self::new_empty(len);
        for it in iter {
            ret.insert(it);
        }
        ret
    }

    /// Returns an iterator over every element in the set.
    pub fn iter(&self) -> impl '_ + Iterator<Item = I> {
        self.bits
            .iter_ones()
            .filter_map(|i| I::try_from_usize(i).ok())
    }

    /// Returns whether `element` is in the set.
    pub fn contains(&self, element: I) -> bool {
        self.bits[element.to_usize()]
    }
    /// Inserts `element` into the set.
    pub fn insert(&mut self, element: I) {
        self.bits.set(element.to_usize(), true)
    }
    /// Removes `element` from the set.
    pub fn remove(&mut self, element: I) {
        self.bits.set(element.to_usize(), false);
    }
}

macro_rules! impl_forward_owned_ops_to_assign_ops {
    ($assign_trait:ident :: $assign_func:ident, $trait:ident :: $func:ident) => {
        impl<I> ::std::ops::$assign_trait<GenericMask<I>> for GenericMask<I> {
            fn $assign_func(&mut self, rhs: Self) {
                ::std::ops::$assign_trait::$assign_func(self, &rhs)
            }
        }
        impl<I> ::std::ops::$trait<&GenericMask<I>> for GenericMask<I> {
            type Output = GenericMask<I>;

            fn $func(mut self, rhs: &GenericMask<I>) -> Self::Output {
                ::std::ops::$assign_trait::$assign_func(&mut self, rhs);
                self
            }
        }
        impl<I> ::std::ops::$trait<GenericMask<I>> for GenericMask<I> {
            type Output = GenericMask<I>;

            fn $func(mut self, rhs: GenericMask<I>) -> Self::Output {
                ::std::ops::$assign_trait::$assign_func(&mut self, rhs);
                self
            }
        }
    };
}

impl<I> Not for GenericMask<I> {
    type Output = GenericMask<I>;

    fn not(mut self) -> Self::Output {
        // This may modify the extra bits at the end of the slice; according to
        // the `bitvec` docs this is fine.
        for data in self.raw_iter_mut() {
            *data = !*data;
        }
        self.clear_uninit()
    }
}
impl<I> Not for &GenericMask<I> {
    type Output = GenericMask<I>;

    fn not(self) -> Self::Output {
        GenericMask::from_raw_iter(self.max_len(), self.raw_iter().map(|data| !data))
    }
}

impl<I> BitOrAssign<&Self> for GenericMask<I> {
    fn bitor_assign(&mut self, rhs: &Self) {
        for (l, r) in std::iter::zip(self.raw_iter_mut(), rhs.raw_iter()) {
            *l |= r;
        }
    }
}
impl_forward_owned_ops_to_assign_ops!(BitOrAssign::bitor_assign, BitOr::bitor);

impl<I> BitAndAssign<&Self> for GenericMask<I> {
    fn bitand_assign(&mut self, rhs: &Self) {
        for (l, r) in std::iter::zip(self.raw_iter_mut(), rhs.raw_iter()) {
            *l &= r;
        }
    }
}
impl_forward_owned_ops_to_assign_ops!(BitAndAssign::bitand_assign, BitAnd::bitand);

impl<I> BitXorAssign<&Self> for GenericMask<I> {
    fn bitxor_assign(&mut self, rhs: &Self) {
        for (l, r) in std::iter::zip(self.raw_iter_mut(), rhs.raw_iter()) {
            *l ^= r;
        }
    }
}
impl_forward_owned_ops_to_assign_ops!(BitXorAssign::bitxor_assign, BitXor::bitxor);
