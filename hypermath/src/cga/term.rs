use std::fmt;
use std::ops::{BitXor, Mul, MulAssign, Neg, Shl};

use crate::*;

/// Term in the conformal geometric algebra, consisting of a real coefficient
/// and a bitmask representing the bases.
#[derive(Debug, Default, Copy, Clone, PartialEq)]
pub struct Term {
    /// Coefficient.
    pub coef: Float,
    /// Bitset of basis blades.
    pub axes: Axes,
}

impl fmt::Display for Term {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.coef, f)?;
        write!(f, " ")?;
        fmt::Display::fmt(&self.axes, f)?;
        Ok(())
    }
}

impl approx::AbsDiffEq for Term {
    type Epsilon = Float;

    fn default_epsilon() -> Self::Epsilon {
        EPSILON
    }

    fn abs_diff_eq(&self, other: &Self, epsilon: Self::Epsilon) -> bool {
        self.axes == other.axes && self.coef.abs_diff_eq(&other.coef, epsilon)
    }
}

/// Negation of a term.
impl Neg for Term {
    type Output = Term;

    fn neg(mut self) -> Self::Output {
        self.coef = -self.coef;
        self
    }
}

/// Geometric product of two terms.
impl Mul for Term {
    type Output = Term;

    fn mul(self, rhs: Self) -> Self::Output {
        let sign = self.axes * rhs.axes;

        Term {
            coef: self.coef * rhs.coef * sign,
            axes: self.axes ^ rhs.axes, // Common axes cancel.
        }
    }
}
impl MulAssign for Term {
    fn mul_assign(&mut self, rhs: Self) {
        *self = *self * rhs;
    }
}

/// Outer product of two terms. Returns `None` when exactly zero.
#[allow(clippy::suspicious_arithmetic_impl)]
impl BitXor for Term {
    type Output = Option<Term>;

    fn bitxor(self, rhs: Self) -> Self::Output {
        (self.axes & rhs.axes).is_empty().then(|| self * rhs)
    }
}

/// Left contraction of two terms. Returns `None` when exactly zero.
///
/// See <https://youtu.be/oVyBbJl6xvo?t=180s> for an intuitive explanation.
impl Shl for Term {
    type Output = Option<Term>;

    fn shl(self, rhs: Self) -> Self::Output {
        rhs.axes.contains(self.axes).then(|| self * rhs)
    }
}

/// Scaling a term by a number.
impl Mul<Float> for Term {
    type Output = Term;

    fn mul(mut self, rhs: Float) -> Self::Output {
        self *= rhs;
        self
    }
}
impl MulAssign<Float> for Term {
    fn mul_assign(&mut self, rhs: Float) {
        self.coef *= rhs;
    }
}

impl Term {
    /// Constructs a scalar term.
    pub const fn scalar(x: Float) -> Self {
        Term {
            coef: x,
            axes: Axes::SCALAR,
        }
    }
    /// Constructs an e₋ term.
    pub const fn e_minus(coef: Float) -> Self {
        Term {
            coef,
            axes: Axes::E_MINUS,
        }
    }
    /// Constructs an e₊ term.
    pub const fn e_plus(coef: Float) -> Self {
        Term {
            coef,
            axes: Axes::E_PLUS,
        }
    }
    /// Constructs a unit term.
    pub const fn unit(axes: Axes) -> Self {
        Term { coef: 1.0, axes }
    }
    /// Constructs a unit pseudoscalar term.
    pub const fn pseudoscalar(ndim: u8) -> Self {
        Self::unit(Axes::pseudoscalar(ndim))
    }
    /// Constructs an inverse pseudoscalar term.
    pub fn inverse_pseudoscalar(ndim: u8) -> Self {
        let pss = Self::pseudoscalar(ndim);
        let sign = pss.axes * pss.axes;
        pss * sign
    }

    /// Returns whether the term is approximately zero.
    pub fn is_zero(self) -> bool {
        approx_eq(&self.coef, &0.0)
    }

    /// Returns the grade of the term, which is the number of basis blades used
    /// to construct it. Every term can be represented as an outer product of no
    /// fewer than _r_ vectors, where _r_ is the term's grade.
    pub const fn grade(self) -> u8 {
        self.axes.count()
    }

    /// Returns the reverse term, which has the axes reversed (which in practice
    /// just means the sign might be flipped).
    #[must_use]
    pub fn reverse(mut self) -> Self {
        self.coef *= self.axes.sign_of_reverse();
        self
    }
    /// Returns the multiplicative inverse, or `None` if it does not exist.
    #[must_use]
    pub fn inverse(&self) -> Option<Self> {
        // Formula from https://math.stackexchange.com/a/556232/1115019
        let rev = self.reverse();
        util::try_div(rev, self.dot(rev))
    }

    /// Returns the scalar (dot) product of two terms.
    pub fn dot(self, other: Self) -> Float {
        if self.axes == other.axes {
            (self * other).coef
        } else {
            0.0
        }
    }
}
