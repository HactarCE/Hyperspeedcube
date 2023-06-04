use std::cmp::Ordering;
use std::fmt;
use std::ops::{Mul, MulAssign, Neg};

use super::Float;

/// Positive or negative.
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, Hash)]
pub enum Sign {
    /// Positive
    #[default]
    Pos = 0,
    /// Negative
    Neg = 1,
}

impl fmt::Display for Sign {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Sign::Pos => write!(f, "+"),
            Sign::Neg => write!(f, "-"),
        }
    }
}

impl Neg for Sign {
    type Output = Self;

    fn neg(self) -> Self::Output {
        match self {
            Sign::Pos => Sign::Neg,
            Sign::Neg => Sign::Pos,
        }
    }
}

impl Mul for Sign {
    type Output = Sign;

    fn mul(self, rhs: Self) -> Self::Output {
        if self == rhs {
            Sign::Pos
        } else {
            Sign::Neg
        }
    }
}
impl MulAssign for Sign {
    fn mul_assign(&mut self, rhs: Self) {
        *self = *self * rhs
    }
}

impl From<Sign> for Float {
    fn from(value: Sign) -> Self {
        value.to_float()
    }
}
impl From<Float> for Sign {
    fn from(value: Float) -> Self {
        match value.is_sign_negative() {
            true => Sign::Neg,
            false => Sign::Pos,
        }
    }
}

impl PartialOrd for Sign {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for Sign {
    fn cmp(&self, other: &Self) -> Ordering {
        self.to_i8().cmp(&other.to_i8())
    }
}

#[allow(missing_docs)]
impl Sign {
    pub fn to_float(self) -> Float {
        self.to_i8() as Float
    }
    pub fn to_i8(self) -> i8 {
        match self {
            Sign::Pos => 1,
            Sign::Neg => -1,
        }
    }
}
