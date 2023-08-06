//! Simple `Sign` type.

use std::cmp::Ordering;
use std::fmt;
use std::ops::{Mul, MulAssign, Neg};

use num_traits::Signed;

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

impl<T: Signed> From<T> for Sign {
    fn from(value: T) -> Self {
        match value.signum().is_negative() {
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
        self.to_num::<i8>().cmp(&other.to_num::<i8>())
    }
}

#[allow(missing_docs)]
impl Sign {
    pub fn to_num<T: Signed>(self) -> T {
        match self {
            Sign::Pos => T::one(),
            Sign::Neg => -T::one(),
        }
    }
}

/// Implements `Mul<Sign>` for a type.
///
/// ```rust
/// #[derive(Debug, Copy, Clone)]
/// struct MyStruct(f32);
/// impl std::ops::Neg for MyStruct {
///     type Output = Self;
///
///     fn neg(self) -> Self::Output {
///         match self {
///             Sign::Pos => self,
///             Sign::Neg => Self(-self.0),
///         }
///     }
/// }
/// impl_mul_sign!(impl Mul<Sign> for MyStruct);
/// impl_mulassign_sign!(impl MulAssign<Sign> for MyStruct);
/// ```
#[macro_export]
macro_rules! impl_mul_sign {
    (impl $($tok:tt)*) => {
        impl $($tok)* {
            type Output = Self;

            fn mul(self, rhs: $crate::Sign) -> Self {
                match rhs {
                    $crate::Sign::Pos => self,
                    $crate::Sign::Neg => -self,
                }
            }
        }
    };
}
/// Implements `MulAssign<Sign>` for a type. See [`impl_mul_sign`] for an
/// example.
#[macro_export]
macro_rules! impl_mulassign_sign {
    (impl $($tok:tt)*) => {
        impl $($tok)* {
            fn mul_assign(&mut self, rhs: $crate::Sign) {
                match rhs {
                    $crate::Sign::Pos => (),
                    $crate::Sign::Neg => *self = -self.clone(),
                }
            }
        }
    };
}

impl_mul_sign!(impl Mul<Sign> for Sign);
impl_mulassign_sign!(impl MulAssign<Sign> for Sign);

impl_mul_sign!(impl Mul<Sign> for f32);
impl_mulassign_sign!(impl MulAssign<Sign> for f32);
impl_mul_sign!(impl Mul<Sign> for f64);
impl_mulassign_sign!(impl MulAssign<Sign> for f64);
