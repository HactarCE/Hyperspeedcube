use itertools::Itertools;
use smallvec::{smallvec, smallvec_inline, SmallVec};
use std::fmt;
use std::ops::{
    Add, AddAssign, BitXor, BitXorAssign, Index, Mul, MulAssign, Neg, Shl, ShlAssign, Sub,
    SubAssign,
};

use super::{Axes, Term};
use crate::math::{approx_eq, util, Matrix, Vector, VectorRef};

/// Sum of terms in the conformal geometric algebra. Terms are stored sorted by
/// their `axes` bitmask. No two terms in one multivector may have the same set
/// of axes.
#[derive(Default, Clone, PartialEq)]
pub struct Multivector(SmallVec<[Term; 2]>);

impl fmt::Display for Multivector {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut terms = self
            .terms()
            .iter()
            .map(|term| term.axes - Axes::E_PLANE)
            .dedup()
            .flat_map(|axes| {
                let scalar_component = self[axes];
                let no_component = self.get_no(axes);
                let ni_component = self.get_ni(axes);
                // E = ∞ ∧ o = - e₋ e₊
                let e_plane_component = -self[axes | Axes::E_PLANE];

                [
                    (format!(" {axes}"), scalar_component),
                    (format!(" nₒ{axes}"), no_component),
                    (format!(" ∞ {axes}"), ni_component),
                    (format!(" E{axes}"), e_plane_component),
                ]
            })
            .filter(|(_axes_string, coef)| !approx_eq(coef, &0.0));

        if let Some((axes_string, coef)) = terms.next() {
            fmt::Display::fmt(&coef, f)?;
            write!(f, "{axes_string}")?;

            for (axes_string, coef) in terms {
                write!(f, " + ")?;

                fmt::Display::fmt(&coef, f)?;
                write!(f, "{axes_string}")?;
            }
        } else {
            fmt::Display::fmt(&0.0_f32, f)?;
        }

        Ok(())
    }
}

impl approx::AbsDiffEq for Multivector {
    type Epsilon = f32;

    fn default_epsilon() -> Self::Epsilon {
        crate::math::EPSILON
    }

    fn abs_diff_eq(&self, other: &Self, epsilon: Self::Epsilon) -> bool {
        let largest_axis_mask = std::cmp::max(self.largest_axis_mask(), other.largest_axis_mask());
        (0..=largest_axis_mask.bits())
            .map(Axes::from_bits_truncate)
            .all(|axes| match (self.get(axes), other.get(axes)) {
                (Some(a), Some(b)) => a.abs_diff_eq(&b, epsilon),
                (Some(n), None) | (None, Some(n)) => n.abs_diff_eq(&0.0, epsilon),
                (None, None) => true,
            })
    }
}

impl From<Term> for Multivector {
    fn from(term: Term) -> Self {
        Self(smallvec::smallvec![term])
    }
}

impl<V: VectorRef> From<V> for Multivector {
    fn from(v: V) -> Self {
        v.iter()
            .enumerate()
            .map(|(i, v)| Term {
                coef: v,
                axes: Axes::euclidean(i as u8),
            })
            .sum()
    }
}

impl fmt::Debug for Multivector {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut ret = f.debug_struct("Multivector");
        for term in &self.0 {
            if term.coef != 0.0 {
                let field_name = if term.axes == Axes::SCALAR {
                    "S".to_string() // scalar
                } else {
                    term.axes.to_string()
                };
                ret.field(&field_name, &term.coef);
            }
        }
        ret.finish()
    }
}

impl Index<Axes> for Multivector {
    type Output = f32;

    fn index(&self, axes: Axes) -> &Self::Output {
        match self.0.binary_search_by_key(&axes, |term| term.axes) {
            Ok(i) => &self.0[i].coef,
            Err(_) => &0.0,
        }
    }
}

/// Negation of multivector.
impl<'a> Neg for &'a Multivector {
    type Output = Multivector;

    fn neg(self) -> Self::Output {
        -self.clone()
    }
}
impl Neg for Multivector {
    type Output = Multivector;

    fn neg(mut self) -> Self::Output {
        for term in &mut self.0 {
            *term = -*term;
        }
        self
    }
}

/// Scaling a multivector by a number.
impl<'a> Mul<f32> for &'a Multivector {
    type Output = Multivector;

    fn mul(self, rhs: f32) -> Self::Output {
        self.clone() * rhs
    }
}
impl Mul<f32> for Multivector {
    type Output = Multivector;

    fn mul(mut self, rhs: f32) -> Self::Output {
        self *= rhs;
        self
    }
}
impl MulAssign<f32> for Multivector {
    fn mul_assign(&mut self, rhs: f32) {
        for term in &mut self.0 {
            term.coef *= rhs;
        }
    }
}

/// Sum of multivector and term.
impl<'a> Add<Term> for &'a Multivector {
    type Output = Multivector;

    fn add(self, rhs: Term) -> Self::Output {
        self.clone() + rhs
    }
}
impl Add<Term> for Multivector {
    type Output = Multivector;

    fn add(mut self, rhs: Term) -> Self::Output {
        self += rhs;
        self
    }
}
impl AddAssign<Term> for Multivector {
    fn add_assign(&mut self, rhs: Term) {
        match self.0.binary_search_by_key(&rhs.axes, |term| term.axes) {
            Ok(i) => {
                self.0[i].coef += rhs.coef;
                // Optimization: remove terms equal to zero.
                if self.0[i].is_zero() {
                    self.0.remove(i);
                }
            }
            Err(i) => self.0.insert(i, rhs),
        }
    }
}

/// Difference of multivector and term.
impl<'a> Sub<Term> for &'a Multivector {
    type Output = Multivector;

    fn sub(self, rhs: Term) -> Self::Output {
        self.clone() - rhs
    }
}
impl Sub<Term> for Multivector {
    type Output = Multivector;

    fn sub(mut self, rhs: Term) -> Self::Output {
        self -= rhs;
        self
    }
}
impl SubAssign<Term> for Multivector {
    fn sub_assign(&mut self, rhs: Term) {
        *self += -rhs;
    }
}

/// Geometric product of multivector and term.
impl<'a> Mul<Term> for &'a Multivector {
    type Output = Multivector;

    fn mul(self, rhs: Term) -> Self::Output {
        self.clone() * rhs
    }
}
impl Mul<Term> for Multivector {
    type Output = Multivector;

    fn mul(mut self, rhs: Term) -> Self::Output {
        self *= rhs;
        self
    }
}
impl<'a> MulAssign<Term> for Multivector {
    fn mul_assign(&mut self, rhs: Term) {
        for term in &mut self.0 {
            *term *= rhs;
        }
        self.0.sort_by_key(|term| term.axes);
    }
}

/// Outer product of a multivector and a term.
///
/// See https://w.wiki/6L8p
impl<'a> BitXor<Term> for &'a Multivector {
    type Output = Multivector;

    fn bitxor(self, rhs: Term) -> Self::Output {
        self.0.iter().flat_map(|&term| term ^ rhs).sum()
    }
}
impl BitXor<Term> for Multivector {
    type Output = Multivector;

    fn bitxor(self, rhs: Term) -> Self::Output {
        &self ^ rhs
    }
}
/// Outer product of a term and a multivector.
///
/// See https://w.wiki/6L8p
impl<'a> BitXor<&'a Multivector> for Term {
    type Output = Multivector;

    fn bitxor(self, rhs: &'a Multivector) -> Self::Output {
        rhs.0.iter().flat_map(|&term| self ^ term).sum()
    }
}
impl BitXor<Multivector> for Term {
    type Output = Multivector;

    fn bitxor(self, rhs: Multivector) -> Self::Output {
        self ^ &rhs
    }
}

/// Left contraction of a term and a multivector.
///
/// See https://youtu.be/oVyBbJl6xvo?t=180s for an intuitive explanation.
impl<'a> Shl<&'a Multivector> for Term {
    type Output = Multivector;

    fn shl(self, rhs: &'a Multivector) -> Self::Output {
        rhs.0.iter().flat_map(|&term| self << term).sum()
    }
}
impl Shl<Multivector> for Term {
    type Output = Multivector;

    fn shl(self, rhs: Multivector) -> Self::Output {
        self << &rhs
    }
}

/// Sum of two multivectors.
impl<'a> Add for &'a Multivector {
    type Output = Multivector;

    fn add(self, rhs: Self) -> Self::Output {
        let mut ret = self.clone();
        for &term in &rhs.0 {
            ret += term;
        }
        ret
    }
}
impl_forward_bin_ops_to_ref! {
    impl Add for Multivector { fn add() }
}
impl<'a> AddAssign<&'a Multivector> for Multivector {
    fn add_assign(&mut self, rhs: &'a Multivector) {
        for &term in &rhs.0 {
            *self += term;
        }
    }
}
impl AddAssign for Multivector {
    fn add_assign(&mut self, rhs: Self) {
        *self += &rhs;
    }
}

/// Difference of two multivectors.
impl<'a> Sub for &'a Multivector {
    type Output = Multivector;

    fn sub(self, rhs: Self) -> Self::Output {
        let mut ret = self.clone();
        for &term in &rhs.0 {
            ret -= term;
        }
        ret
    }
}
impl_forward_bin_ops_to_ref! {
    impl Sub for Multivector { fn sub() }
}
impl<'a> SubAssign<&'a Multivector> for Multivector {
    fn sub_assign(&mut self, rhs: &'a Multivector) {
        for &term in &rhs.0 {
            *self -= term;
        }
    }
}
impl SubAssign for Multivector {
    fn sub_assign(&mut self, rhs: Self) {
        *self -= &rhs;
    }
}

/// Geometric product of two multivectors.
impl<'a> Mul for &'a Multivector {
    type Output = Multivector;

    fn mul(self, rhs: Self) -> Self::Output {
        let mut ret = Multivector::zero();
        for &a in &self.0 {
            for &b in &rhs.0 {
                ret += a * b;
            }
        }
        ret
    }
}
impl_forward_bin_ops_to_ref! {
    impl Mul for Multivector { fn mul() }
}
impl_forward_assign_ops_to_owned! {
    impl MulAssign for Multivector { fn mul_assign() { * } }
}

/// Outer product of two multivectors.
///
/// See https://w.wiki/6L8p
impl<'a> BitXor for &'a Multivector {
    type Output = Multivector;

    fn bitxor(self, rhs: Self) -> Self::Output {
        let mut ret = Multivector::zero();
        for &a in &self.0 {
            for &b in &rhs.0 {
                // Don't bother computing a term if it would get grade-projected
                // out at the end.
                if let Some(new_term) = a ^ b {
                    ret += new_term;
                }
            }
        }
        ret
    }
}
impl_forward_bin_ops_to_ref! {
    impl BitXor for Multivector { fn bitxor() }
}
impl_forward_assign_ops_to_owned! {
    impl BitXorAssign for Multivector { fn bitxor_assign() { ^ } }

}

/// Left contraction of two multivectors.
///
/// See https://youtu.be/oVyBbJl6xvo?t=180s for an intuitive explanation.
impl<'a> Shl for &'a Multivector {
    type Output = Multivector;

    fn shl(self, rhs: Self) -> Self::Output {
        let mut ret = Multivector::zero();
        for &a in &self.0 {
            for &b in &rhs.0 {
                // Don't bother computing a term if it would get grade-projected
                // out at the end.
                if let Some(new_term) = a << b {
                    ret += new_term;
                }
            }
        }
        ret
    }
}
impl_forward_bin_ops_to_ref! {
    impl Shl for Multivector { fn shl() }
}
impl_forward_assign_ops_to_owned! {
    impl ShlAssign for Multivector { fn shl_assign() { << } }

}

/// Sum of multivectors.
impl std::iter::Sum for Multivector {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.fold(Multivector::ZERO, |a, b| a + b)
    }
}

/// Sum of terms into multivector.
impl std::iter::Sum<Term> for Multivector {
    fn sum<I: Iterator<Item = Term>>(iter: I) -> Self {
        iter.fold(Multivector::ZERO, |a, b| a + b)
    }
}

impl Multivector {
    /// Zero multivector.
    pub const ZERO: Self = Self(SmallVec::new_const());

    /// Null vector representing the origin.
    ///
    /// See https://w.wiki/6L8q
    pub const NO: Self = Self(smallvec_inline![Term::e_minus(0.5), Term::e_plus(-0.5)]);
    /// Null vector representing the point at infinity.
    ///
    /// See https://w.wiki/6L8q
    pub const NI: Self = Self(smallvec_inline![Term::e_minus(1.0), Term::e_plus(1.0)]);

    /// Returns a scalar multivector.
    pub fn scalar(s: f32) -> Self {
        Multivector(smallvec![Term::scalar(s)])
    }
    /// Returns the zero multivector.
    pub const fn zero() -> Self {
        Self::ZERO
    }
    /// Returns the identity multivector.
    pub fn identity() -> Self {
        Self::scalar(1.0)
    }
    /// Returns the Minkowski plane, defined as E=∞∧o.
    pub fn minkowski_plane() -> Self {
        Multivector(smallvec![-Term::unit(Axes::E_PLANE)])
    }

    /// Returns the lexicographically largest axis mask in the multivector.
    fn largest_axis_mask(&self) -> Axes {
        match self.0.last() {
            Some(term) => term.axes,
            None => Axes::empty(),
        }
    }
    /// Returns the minimum number of Euclidean dimensions that this multivector
    /// requires.
    pub fn ndim(&self) -> u8 {
        // Subtract two to account for e₋ and e₊ using the least significant
        // bits of the axis mask.
        self.largest_axis_mask().min_euclidean_ndim()
    }
    /// Returns a term of the multivector, or `None` if it is zero.
    pub fn get(&self, axes: Axes) -> Option<f32> {
        // TODO(optimization): linear search may be faster, especially for
        //                     smaller indices
        self.0
            .binary_search_by_key(&axes, |term| term.axes)
            .ok()
            .map(|i| self.0[i].coef)
    }
    /// Returns the component of the multivector parallel to No times
    /// `other_axes`.
    pub fn get_no(&self, other_axes: Axes) -> f32 {
        self[other_axes | Axes::E_MINUS] - self[other_axes | Axes::E_PLUS]
    }
    /// Returns the component of the multivector parallel to Ni times
    /// `other_axes`.
    pub fn get_ni(&self, other_axes: Axes) -> f32 {
        (self[other_axes | Axes::E_MINUS] + self[other_axes | Axes::E_PLUS]) / 2.0
    }

    /// Returns whether the multivector approximately equals zero.
    pub fn is_zero(&self) -> bool {
        self.terms().iter().all(|term| term.is_zero())
    }

    /// Returns the terms of the multivector.
    pub fn terms(&self) -> &[Term] {
        &self.0
    }

    /// Returns the reverse multivector, which has all the same terms but with
    /// the axes reversed (which in practice just means some signs are flipped).
    #[must_use]
    pub fn reverse(&self) -> Self {
        Multivector(self.0.iter().copied().map(Term::reverse).collect())
    }
    /// Returns the multiplicative inverse, or `None` if it does not exist.
    #[must_use]
    pub fn inverse(&self) -> Option<Self> {
        // Formula from https://math.stackexchange.com/a/556232/1115019
        let rev = self.reverse();
        util::try_div(&rev, self.dot(&rev))
    }

    /// Returns the scalar (dot) product of two multivectors.
    pub fn dot(&self, other: &Multivector) -> f32 {
        let mut ret = 0.0;

        let mut self_terms = self.terms().iter().peekable();
        let mut other_terms = other.terms().iter().peekable();
        while let (Some(&&a), Some(&&b)) = (self_terms.peek(), other_terms.peek()) {
            match a.axes.cmp(&b.axes) {
                std::cmp::Ordering::Less => {
                    self_terms.next();
                }
                std::cmp::Ordering::Greater => {
                    other_terms.next();
                }
                std::cmp::Ordering::Equal => {
                    ret += (a * b).coef;
                    self_terms.next();
                    other_terms.next();
                }
            }
        }

        ret
    }

    /// Returns the sandwich product with a vector: `M * v * M_rev`.
    pub fn sandwich_vector(&self, v: impl VectorRef) -> Vector {
        let ndim = std::cmp::max(self.ndim(), v.ndim());
        (0..ndim)
            .map(|i| self.sandwich_axis_vector(i, v.get(i)))
            .sum()
    }
    /// Returns the sandwich product with a multivector: `M * R * M_rev`.
    #[must_use]
    pub fn sandwich_multivector(&self, multivector: &Multivector) -> Multivector {
        self.0
            .iter()
            .flat_map(|&lhs| {
                multivector
                    .0
                    .iter()
                    .flat_map(move |&mid| self.0.iter().map(move |&rhs| lhs * mid * rhs.reverse()))
            })
            .sum()
    }
    /// Returns the matrix equivalent to a sandwich product with the
    /// multivector.
    ///
    /// The matix is more expensive to compute initially than any one sandwich
    /// product, but cheaper to apply.
    pub fn matrix(&self) -> Matrix {
        Matrix::from_cols((0..self.ndim()).map(|axis| self.sandwich_axis_vector(axis, 1.0)))
    }
    /// Returns the sandwich product with an axis-aligned vector: `M * v
    /// * M_rev`.
    fn sandwich_axis_vector(&self, axis: u8, mag: f32) -> Vector {
        let ndim = std::cmp::max(self.ndim(), axis + 1);
        let mid = Term {
            coef: mag,
            axes: Axes::euclidean(axis),
        };

        let mut ret = Vector::zero(ndim);
        for &lhs in &self.0 {
            for &rhs in &self.0 {
                let term = lhs * mid * rhs.reverse();
                if term.axes.count() == 1 {
                    let euclidean_axis = term.axes.min_euclidean_ndim() - 1;
                    ret[euclidean_axis] += term.coef;
                }
            }
        }
        ret
    }

    /// Returns a multivector containing a subset of the terms of it.
    #[must_use]
    pub fn filter_terms(&self, mut f: impl FnMut(Axes) -> bool) -> Multivector {
        Multivector(self.0.iter().copied().filter(|term| f(term.axes)).collect())
    }
    /// Returns a grade projection of the multivector.
    #[must_use]
    pub fn grade_project(&self, grade: u8) -> Multivector {
        self.filter_terms(|term| term.count() == grade)
    }
    /// Returns the maximum grade of the multivector.
    pub fn max_grade(&self) -> u8 {
        self.0.iter().map(|term| term.grade()).max().unwrap_or(0)
    }

    /// Returns the magnitude (square root of sum of squares) of the
    /// multivector.
    pub fn mag(&self) -> f32 {
        self.0.iter().map(|term| term.coef * term.coef).sum()
    }
    /// Normalizes the multivector so that the magnitude is one, or returns
    /// `None` if the multivector is zero.
    #[must_use]
    pub fn normalize(&self) -> Option<Multivector> {
        let mult = 1.0 / self.mag();
        mult.is_finite().then(|| self * mult)
    }

    /// Returns the axis mask of the term with the greatest absolute value.
    pub fn most_significant_term(&self) -> Axes {
        self.0
            .iter()
            .max_by(|a, b| f32::total_cmp(&a.coef.abs(), &b.coef.abs()))
            .map(|term| term.axes)
            .unwrap_or(Axes::SCALAR)
    }
}
