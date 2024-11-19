use std::fmt;
use std::ops::Mul;

use bitflags::bitflags;
use itertools::Itertools;

use crate::Float;

bitflags! {
    /// Set of axes for a term in the conformal geometric algebra.
    #[derive(Debug, Default, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
    pub struct Axes: u16 {
        /// Scalar (no axes)
        const SCALAR = 0;

        /// e₋
        const E_MINUS = 1 << 0;
        /// e₊
        const E_PLUS = 1 << 1;
        /// Minkowski plane
        const E_PLANE = Self::E_MINUS.bits() | Self::E_PLUS.bits();

        /// Euclidean X axis (0)
        const X = 1 << 2;
        /// Euclidean Y axis (1)
        const Y = 1 << 3;
        /// Euclidean Z axis (2)
        const Z = 1 << 4;
        /// Euclidean W axis (3)
        const W = 1 << 5;
        /// Euclidean V axis (4)
        const V = 1 << 6;
        /// Euclidean U axis (5)
        const U = 1 << 7;
        /// Euclidean T axis (6)
        const T = 1 << 8;
        /// Euclidean S axis (7)
        const S = 1 << 9;
        /// Euclidean R axis (8)
        const R = 1 << 10;
        /// Euclidean Q axis (9)
        const Q = 1 << 11;
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
        Self::from_bits_truncate(x as u16)
    }

    fn to_u64(self) -> u64 {
        self.bits() as u64
    }
}

impl Axes {
    /// Human-friendly name of each axis.
    pub const NAMES: &'static [&'static str] =
        &["e₋", "e₊", "x", "y", "z", "w", "v", "u", "t", "s", "r"];

    /// Returns the `i`th Euclidean axis (zero-indexed).
    pub const fn euclidean(i: u8) -> Self {
        Self::from_bits_truncate(1 << (i + 2))
    }
    /// Returns the set of all axes for N-dimensional Euclidean space.
    pub const fn pseudoscalar(ndim: u8) -> Self {
        Self::from_bits_truncate((1 << (ndim + 2)) - 1) // +2 to skip e₋ and e₊
    }

    /// Returns the sign of the reverse of the basis blade.
    pub const fn sign_of_reverse(self) -> Float {
        // The number of swaps required to reverse a sequence of length n is
        // n*(n+1)/2. See <https://oeis.org/A000217>. This sequence alternates
        // between pairs of even and odd numbers; if its parity is odd, then
        // negate the coefficient.
        match self.bits().count_ones() % 4 {
            0 | 1 => 1.0,
            2 | 3 => -1.0,
            _ => unreachable!(),
        }
    }

    /// Returns the number of axes.
    pub const fn count(self) -> u8 {
        self.bits().count_ones() as _
    }

    /// Returns the minimum number of Euclidean dimensions required to represent
    /// this set of axes.
    pub fn min_euclidean_ndim(self) -> u8 {
        let bits = self.bits();
        let total_bits = size_of_val(&bits) as u32 * 8;
        (total_bits - 2).saturating_sub(bits.leading_zeros()) as u8
    }
    /// Returns the Euclidean axis, if it is exactly one axis and that axis is
    /// Euclidean.
    pub fn single_euclidean_axis(self) -> Option<u8> {
        (self.count() == 1).then(|| self.min_euclidean_ndim())
    }
}

/// Returns the sign of the geometric product between two basis blades.
impl Mul for Axes {
    type Output = Float;

    fn mul(self, rhs: Self) -> Self::Output {
        // NOTE: If this is a performance bottleneck, it should be easy enough to
        // make a macro that produces a lookup table for this function.

        // Count the number of swaps needed to sort the combined product. If the
        // number of swaps is odd, negate the result.
        let mut neg = false;
        let mut a = self.bits();
        let mut b = rhs.bits();
        while a != 0 && b != 0 {
            let i = b.trailing_zeros() + 1;
            a >>= i;
            b >>= i;
            neg ^= a.count_ones() & 1 != 0;
        }

        // e₋ squares to -1.
        neg ^= (self & rhs).contains(Self::E_MINUS);

        if neg {
            -1.0
        } else {
            1.0
        }
    }
}
