use std::fmt;
use std::ops::{BitXor, Mul, MulAssign, Neg, Shl};

use super::Axes;
use crate::*;

/// Term in the projective geometric algebra, consisting of a real coefficient
/// and a bitmask representing the bases.
///
/// This struct isn't stored anywhere; it's mostly just construrcted temporarily
/// for iteration over the terms of a multivector.
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

/// Geometric product of two terms. Returns `None` when exactly zero.
impl Mul for Term {
    type Output = Option<Term>;

    fn mul(self, rhs: Self) -> Self::Output {
        Self::geometric_product(self, rhs)
    }
}

/// Outer product of two terms. Returns `None` when exactly zero.
#[allow(clippy::suspicious_arithmetic_impl)]
impl BitXor for Term {
    type Output = Option<Term>;

    fn bitxor(self, rhs: Self) -> Self::Output {
        (self.axes & rhs.axes)
            .is_empty()
            .then(|| self * rhs)
            .flatten()
    }
}

/// Left contraction of two terms. Returns `None` when exactly zero.
///
/// See <https://youtu.be/oVyBbJl6xvo?t=180s> for an intuitive explanation.
impl Shl for Term {
    type Output = Option<Term>;

    fn shl(self, rhs: Self) -> Self::Output {
        rhs.axes.contains(self.axes).then(|| self * rhs).flatten()
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
    /// Constructs an e₀ term.
    pub const fn e0(coef: Float) -> Self {
        Term {
            coef,
            axes: Axes::E0,
        }
    }
    /// Constructs a unit term.
    pub const fn unit(axes: Axes) -> Self {
        Term { coef: 1.0, axes }
    }
    /// Constructs a unit scalar term.
    pub const fn scalar_unit() -> Self {
        Self::unit(Axes::SCALAR)
    }
    /// Constructs a unit pseudoscalar term in `ndim`-dimensional space.
    pub const fn antiscalar_unit(ndim: u8) -> Self {
        Self::unit(Axes::antiscalar(ndim))
    }

    /// Returns whether the term is approximately zero.
    pub fn is_zero(self) -> bool {
        approx_eq(&self.coef, &0.0)
    }

    /// Returns the grade of the term, which is the number of basis blades used
    /// to construct it. Every term can be represented as an outer product of no
    /// fewer than _r_ vectors, where _r_ is the term's grade.
    pub const fn grade(self) -> u8 {
        self.axes.grade()
    }
    /// Returns the antigrade of the term, which is `ndim` minus the grade.
    pub const fn antigrade(self, ndim: u8) -> u8 {
        ndim + 1 - self.grade() // +1 because `ndim` doesn't include e₀
    }

    /// Returns the reverse term, which has the axes reversed (which in practice
    /// just means the sign might be flipped).u
    #[must_use]
    pub fn reverse(mut self) -> Self {
        self.coef *= self.axes.sign_of_reverse();
        self
    }
    /// Returns the antireverse term in `ndim`-dimensional space.
    #[must_use]
    pub fn antireverse(mut self, ndim: u8) -> Self {
        self.coef *= self.axes.sign_of_antireverse(ndim);
        self
    }
    /// Returns the [geometric product] between `lhs` and `rhs`, or `None` if
    /// the result is zero.
    ///
    /// [geometric product]:
    ///     https://rigidgeometricalgebra.org/wiki/index.php?title=Geometric_products
    #[must_use]
    pub fn geometric_product(lhs: Self, rhs: Self) -> Option<Self> {
        let sign = Axes::sign_of_geometric_product(lhs.axes, rhs.axes)?;
        Some(Term {
            coef: lhs.coef * rhs.coef * sign,
            axes: Axes::unsigned_geometric_product(lhs.axes, rhs.axes),
        })
    }
    /// Returns the [geometric antiproduct] between `lhs` and `rhs` in
    /// `ndim`-dimensional space, or `None` if the result is zero.
    ///
    /// [geometric antiproduct]:
    ///     https://rigidgeometricalgebra.org/wiki/index.php?title=Geometric_products
    #[must_use]
    pub fn geometric_antiproduct(lhs: Self, rhs: Self, ndim: u8) -> Option<Self> {
        let sign = Axes::sign_of_geometric_antiproduct(lhs.axes, rhs.axes, ndim)?;
        Some(Term {
            coef: lhs.coef * rhs.coef * sign,
            axes: Axes::unsigned_geometric_product(lhs.axes, rhs.axes),
        })
    }
    /// Returns the [right complement] of the term in `ndim`-dimensional space.
    ///
    /// [right complement]:
    ///     https://rigidgeometricalgebra.org/wiki/index.php?title=Complements
    #[must_use]
    pub fn right_complement(self, ndim: u8) -> Term {
        let sign = self.axes.sign_of_right_complement(ndim);
        Term {
            coef: self.coef * sign,
            axes: self.axes.unsigned_complement(ndim),
        }
    }
    /// Returns the [left complement] of the term in `ndim`-dimensional space.
    ///
    /// [left complement]:
    ///     https://rigidgeometricalgebra.org/wiki/index.php?title=Complements
    #[must_use]
    pub fn left_complement(self, ndim: u8) -> Term {
        let sign = self.axes.sign_of_left_complement(ndim);
        Term {
            coef: self.coef * sign,
            axes: self.axes.unsigned_complement(ndim),
        }
    }

    /// Returns the [exterior product] between `lhs` and `rhs`, or `None` if the
    /// result is zero.
    ///
    /// [exterior product]:
    ///     https://rigidgeometricalgebra.org/wiki/index.php?title=Exterior_products#Exterior_Product
    #[must_use]
    pub fn wedge(lhs: Self, rhs: Self) -> Option<Self> {
        // Exterior product is zero if there are any basis blades in common.
        if (lhs.axes & rhs.axes).is_empty() {
            Self::geometric_product(lhs, rhs)
        } else {
            None
        }
    }
    /// Returns the scalar [dot product] between `lhs` and `rhs`, or `None` if
    /// the result is zero.
    ///
    /// [dot product]:
    ///     https://rigidgeometricalgebra.org/wiki/index.php?title=Dot_products#Dot_Product
    #[must_use]
    pub fn dot(lhs: Self, rhs: Self) -> Option<Self> {
        // Interior product is zero unless all basis blades are the same.
        if lhs.axes == rhs.axes {
            Self::geometric_product(lhs, rhs)
        } else {
            None
        }
    }

    /// Returns the [metric dual] of the term in `ndim`-dimensional space, or
    /// `None` if it is zero.
    ///
    /// [metric dual]:
    ///     https://rigidgeometricalgebra.org/wiki/index.php?title=Duals#Dual
    #[must_use]
    pub fn dual(self, ndim: u8) -> Option<Self> {
        if self.axes.contains(Axes::E0) {
            None
        } else {
            Some(self.right_complement(ndim))
        }
    }
    /// Returns the [metric antidual] of the term in `ndim`-dimensional space,
    /// or `None` if it is zero.
    ///
    /// [metric antidual]:
    ///     https://rigidgeometricalgebra.org/wiki/index.php?title=Duals#Antidual
    #[must_use]
    pub fn antidual(self, ndim: u8) -> Option<Self> {
        if self.axes.contains(Axes::E0) {
            Some(self.right_complement(ndim))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_term_complements() {
        for ndim in 1..=7 {
            let pss = Term::antiscalar_unit(ndim);
            for i in 0..=ndim {
                let term = Term::antiscalar_unit(i);
                let grade = term.grade();
                let lc = term.left_complement(ndim);
                let rc = term.right_complement(ndim);

                let expected_sign = (-1.0 as Float).powf((grade * term.antigrade(ndim)) as _);

                // Complements are the inverse of term
                assert_eq!(term * rc, Some(pss));
                assert_eq!(lc * term, Some(pss));

                // Complement is self-inverse up to a sign flip
                assert_eq!(lc, rc * expected_sign);
                assert_eq!(rc.right_complement(ndim), term * expected_sign);
                assert_eq!(lc.left_complement(ndim), term * expected_sign);

                // Complement operations are inverses of each other
                assert_eq!(lc.right_complement(ndim), term);
                assert_eq!(rc.left_complement(ndim), term);
            }
        }
    }
}
