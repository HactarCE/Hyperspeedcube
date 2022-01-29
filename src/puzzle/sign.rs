//! Sign enum.

use std::ops::{Add, Mul, Neg};

/// Positive, negative, or zero.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Sign {
    /// Negative.
    Neg = -1,
    /// Zero.
    Zero = 0,
    /// Positive.
    Pos = 1,
}
impl Default for Sign {
    fn default() -> Sign {
        Sign::Zero
    }
}
impl Neg for Sign {
    type Output = Sign;
    fn neg(self) -> Sign {
        match self {
            Sign::Neg => Sign::Pos,
            Sign::Zero => Sign::Zero,
            Sign::Pos => Sign::Neg,
        }
    }
}
impl Mul<Sign> for Sign {
    type Output = Sign;
    fn mul(self, rhs: Sign) -> Sign {
        match self {
            Sign::Neg => -rhs,
            Sign::Zero => Sign::Zero,
            Sign::Pos => rhs,
        }
    }
}
impl Add<Sign> for Sign {
    type Output = Sign;
    fn add(self, rhs: Sign) -> Sign {
        match self {
            Sign::Neg => match rhs {
                Sign::Neg => panic!("too negative"),
                Sign::Zero => Sign::Neg,
                Sign::Pos => Sign::Zero,
            },
            Sign::Zero => rhs,
            Sign::Pos => match rhs {
                Sign::Neg => Sign::Zero,
                Sign::Zero => Sign::Pos,
                Sign::Pos => panic!("too positive"),
            },
        }
    }
}
impl Sign {
    /// Returns an integer representation of the sign (either -1, 0, or 1).
    pub const fn int(self) -> isize {
        match self {
            Sign::Neg => -1,
            Sign::Zero => 0,
            Sign::Pos => 1,
        }
    }
    /// Returns a floating-point representation of the sign (either -1.0, 0.0,
    /// or 1.0).
    pub const fn float(self) -> f32 {
        self.int() as f32
    }
    /// Returns the absolute value of the integer representation of the sign
    /// (either 0 or 1).
    pub const fn abs(self) -> usize {
        match self {
            Sign::Neg | Sign::Pos => 1,
            Sign::Zero => 0,
        }
    }
    /// Returns true if `Sign::Zero` or false otherwise.
    pub const fn is_zero(self) -> bool {
        matches!(self, Sign::Zero)
    }
    /// Returns false if `Sign::Zero` or true otherwise.
    pub const fn is_nonzero(self) -> bool {
        !self.is_zero()
    }

    /// Returns an iterator over all signs.
    pub fn iter() -> impl Clone + Iterator<Item = Sign> {
        [Sign::Neg, Sign::Zero, Sign::Pos].into_iter()
    }
}
