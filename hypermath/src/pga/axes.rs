use std::fmt;

use bitflags::bitflags;
use itertools::Itertools;

use crate::Sign;

bitflags! {
    /// Set of axes for a term in the projective geometric algebra.
    #[derive(Debug, Default, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
    pub struct Axes: u8 {
        /// Scalar (no axes)
        const SCALAR = 0;

        /// Null vector e₀
        const E0 = 1 << 0;

        /// Euclidean X axis (0)
        const X = 1 << 1;
        /// Euclidean Y axis (1)
        const Y = 1 << 2;
        /// Euclidean Z axis (2)
        const Z = 1 << 3;
        /// Euclidean W axis (3)
        const W = 1 << 4;
        /// Euclidean V axis (4)
        const V = 1 << 5;
        /// Euclidean U axis (5)
        const U = 1 << 6;
        /// Euclidean T axis (6)
        const T = 1 << 7;
    }
}

impl fmt::Display for Axes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for i in std::iter::successors(Some(self.bits()), |a| Some(a >> 1))
            .take_while(|&a| a != 0)
            .positions(|a| a & 1 != 0)
        {
            write!(f, "{}", Axes::NAMES.get(i).copied().unwrap_or("?"))?;
        }
        Ok(())
    }
}

impl tinyset::Fits64 for Axes {
    unsafe fn from_u64(x: u64) -> Self {
        Self::from_bits_truncate(x as u8)
    }

    fn to_u64(self) -> u64 {
        self.bits() as u64
    }
}

impl Axes {
    /// Human-friendly name of each axis.
    pub const NAMES: &'static [&'static str] = &["e₀", "x", "y", "z", "w", "v", "u", "t"];

    /// Returns the `i`th Euclidean axis (zero-indexed).
    pub const fn euclidean(i: u8) -> Self {
        Self::from_bits_truncate(1 << (i + 1))
    }
    /// Returns the set of all axes for N-dimensional Euclidean space.
    pub const fn antiscalar(ndim: u8) -> Self {
        Self::from_bits_truncate(((1 << (ndim as u16 + 1)) - 1) as u8) // +1 because e₀ exists
    }

    /// Returns the sign of the [reverse] of the basis blade.
    ///
    /// [reverse]:
    ///     https://rigidgeometricalgebra.org/wiki/index.php?title=Reverses
    pub fn sign_of_reverse(self) -> Sign {
        // The number of swaps required to reverse a sequence of length n is
        // n*(n+1)/2. See <https://oeis.org/A000217>. This sequence alternates
        // between pairs of even and odd numbers; if its parity is odd, then
        // negate the coefficient.
        match self.bits().count_ones() % 4 {
            0 | 1 => Sign::Pos,
            2 | 3 => Sign::Neg,
            _ => unreachable!(),
        }
    }
    /// Returns the sign of the [antireverse] of the basis blade in
    /// `ndim`-dimensional space.
    ///
    /// [antireverse]:
    ///     https://rigidgeometricalgebra.org/wiki/index.php?title=Reverses
    pub fn sign_of_antireverse(self, ndim: u8) -> Sign {
        self.unsigned_complement(ndim).sign_of_reverse()
    }
    /// Returns the sign of the [geometric product] between `lhs` and `rhs`, or
    /// `None` if the result is zero.
    ///
    /// [geometric product]:
    ///     https://rigidgeometricalgebra.org/wiki/index.php?title=Geometric_products
    pub fn sign_of_geometric_product(lhs: Self, rhs: Self) -> Option<Sign> {
        // NOTE: If this is a performance bottleneck, it should be easy enough
        // to make a macro that produces a lookup table for this function.

        // e₀ squares to 0.
        if lhs.contains(Self::E0) && rhs.contains(Self::E0) {
            return None;
        }

        // Count the number of swaps needed to sort the combined product. If the
        // number of swaps is odd, negate the result.
        let mut sign = Sign::Pos;
        let mut a = lhs.bits();
        let mut b = rhs.bits();
        while a != 0 && b & 0x7F != 0 {
            let i = b.trailing_zeros() + 1;
            a >>= i;
            b >>= i;
            if a.count_ones() & 1 != 0 {
                sign = -sign;
            }
        }

        Some(sign)
    }
    /// Returns the sign of the [geometric antiproduct] between `lhs` and `rhs`
    /// in `ndim`-dimensional space.
    ///
    /// [geometric antiproduct]:
    ///     https://rigidgeometricalgebra.org/wiki/index.php?title=Geometric_products
    pub fn sign_of_geometric_antiproduct(lhs: Self, rhs: Self, ndim: u8) -> Option<Sign> {
        let lc = lhs.unsigned_complement(ndim);
        let rc = rhs.unsigned_complement(ndim);

        let mut ret = Sign::Pos;
        // Use De Morgan's laws: take the right complement of each argument ...
        ret *= lhs.sign_of_right_complement(ndim);
        ret *= rhs.sign_of_right_complement(ndim);
        // ... then geometric-product them together ...
        ret *= Self::sign_of_geometric_product(lc, rc)?;
        ret *= (lc ^ rc).sign_of_left_complement(ndim);
        // ... then return the sign flips after doing all of that.
        Some(ret)
    }
    /// Returns the sign of the [right complement] of the basis blade in
    /// `ndim`-dimensional space.
    ///
    /// [right complement]:
    ///     https://rigidgeometricalgebra.org/wiki/index.php?title=Complements
    pub fn sign_of_right_complement(self, ndim: u8) -> Sign {
        let complement = self.unsigned_complement(ndim);
        Self::sign_of_geometric_product(self, complement)
            .expect("right complement should never be zero")
    }
    /// Returns the sign of the [left complement] of the basis blade in
    /// `ndim`-dimensional space, or `None` if the result is zero.
    ///
    /// [left complement]:
    ///     https://rigidgeometricalgebra.org/wiki/index.php?title=Complements
    pub fn sign_of_left_complement(self, ndim: u8) -> Sign {
        let complement = self.unsigned_complement(ndim);
        Self::sign_of_geometric_product(complement, self)
            .expect("left complement should never be zero")
    }
    /// Returns the unsigned geometric product of `lhs` and `rhs`.
    pub fn unsigned_geometric_product(lhs: Self, rhs: Self) -> Axes {
        lhs ^ rhs
    }
    /// Returns the unsigned complement of the basis blade.
    pub fn unsigned_complement(self, ndim: u8) -> Axes {
        self ^ Self::antiscalar(ndim)
    }

    /// Returns the number of basis blades.
    pub const fn grade(self) -> u8 {
        self.bits().count_ones() as _
    }
    /// Returns the number of basis blades in the complement, in
    /// `ndim`-dimensional space.
    pub const fn antigrade(self, ndim: u8) -> u8 {
        ndim + 1 - self.grade() // +1 because `ndim` doesn't include e₀
    }

    /// Returns the Euclidean axis, if it is exactly one axis and that axis is
    /// Euclidean.
    pub fn single_euclidean_axis(self) -> Option<u8> {
        if self.grade() != 1 {
            return None;
        }
        let euclidean_bits = self.bits() >> 1;
        if euclidean_bits == 0 {
            return None;
        }
        let total_bits = size_of_val(&euclidean_bits) as u32 * 8;
        Some((total_bits - 1 - euclidean_bits.leading_zeros()) as u8)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_single_euclidean_axis() {
        assert_eq!(Axes::E0.single_euclidean_axis(), None);
        assert_eq!(Axes::X.single_euclidean_axis(), Some(0));
        assert_eq!(Axes::Y.single_euclidean_axis(), Some(1));
        assert_eq!(Axes::Z.single_euclidean_axis(), Some(2));
        assert_eq!(Axes::T.single_euclidean_axis(), Some(6));
    }
}
