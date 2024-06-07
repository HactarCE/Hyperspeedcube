use std::fmt;
use std::ops::{Add, AddAssign, Index, IndexMut, Mul, Neg, Sub, SubAssign};

use float_ord::FloatOrd;

use super::{Axes, Term};
use crate::{approx_eq, is_approx_nonzero, Float, Hyperplane, Vector, VectorRef};

/// Sum of terms of the same grade in the projective geometric algebra.
#[derive(Clone, PartialEq)]
pub struct Blade {
    /// Number of dimensions that the blade exists in.
    ndim: u8,
    /// Grade of the blade.
    grade: u8,
    /// Coefficients of the terms of the multivector, ordered by the `Axes`
    /// values they correspond to.
    coefficients: Box<[Float]>,
}

impl fmt::Debug for Blade {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut ret = f.debug_struct("Blade");
        super::debug_multivector_struct_fields(&mut ret, self.terms());
        ret.finish()
    }
}

impl fmt::Display for Blade {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        super::display_multivector(f, self.terms())
    }
}

impl approx::AbsDiffEq for Blade {
    type Epsilon = Float;

    fn default_epsilon() -> Self::Epsilon {
        crate::EPSILON
    }

    fn abs_diff_eq(&self, other: &Self, epsilon: Self::Epsilon) -> bool {
        self.ndim == other.ndim
            && self.grade == other.grade
            && std::iter::zip(&self.coefficients[..], &other.coefficients[..])
                .all(|(c1, c2)| approx::AbsDiffEq::abs_diff_eq(c1, c2, epsilon))
    }
}

impl Index<Axes> for Blade {
    type Output = Float;

    fn index(&self, index: Axes) -> &Self::Output {
        self.get(index).expect("bad blade index")
    }
}
impl IndexMut<Axes> for Blade {
    fn index_mut(&mut self, index: Axes) -> &mut Self::Output {
        self.get_mut(index).expect("bad index")
    }
}

impl Blade {
    /// Constructs a new zero blade of grade `grade` in `ndim` dimensions.
    pub fn zero(ndim: u8, grade: u8) -> Self {
        let len = super::multivector_term_order(ndim, grade).len();
        Self {
            ndim,
            grade,
            coefficients: vec![0.0; len].into_boxed_slice(),
        }
    }
    /// Constructs a unit blade of grade 0 in `ndim` dimensions.
    pub fn one(ndim: u8) -> Self {
        Self::scalar(ndim, 1.0)
    }
    /// Constructs a blade of grade 0 in `ndim` dimensions.
    pub fn scalar(ndim: u8, value: Float) -> Self {
        Self {
            ndim,
            grade: 0,
            coefficients: vec![value].into_boxed_slice(),
        }
    }
    /// Constructs a blade from a single term.
    pub fn from_term(ndim: u8, term: Term) -> Self {
        let mut ret = Self::zero(ndim, term.grade());
        ret[term.axes] = term.coef;
        ret
    }

    /// Constructs a blade representing the point at the origin.
    pub fn origin(ndim: u8) -> Self {
        let mut ret = Self::zero(ndim, 1);
        ret[Axes::E0] = 1.0;
        ret
    }
    /// Constructs a blade from a point.
    pub fn from_point(ndim: u8, v: impl VectorRef) -> Self {
        let mut ret = Self::from_vector(ndim, v);
        ret[Axes::E0] = 1.0;
        ret
    }
    /// Extracts the point represented by a blade.
    pub fn to_point(&self) -> Option<Vector> {
        crate::util::try_div(self.to_vector()?, self[Axes::E0])
    }
    /// Returns whether the blade represents a point.
    pub fn is_point(&self) -> bool {
        self.grade == 1 && is_approx_nonzero(&self[Axes::E0])
    }

    /// Constructs a blade from a vector.
    pub fn from_vector(ndim: u8, v: impl VectorRef) -> Self {
        let mut ret = Self::zero(ndim, 1);
        for (i, x) in v.iter_ndim(ndim).enumerate() {
            ret[Axes::euclidean(i as u8)] = x;
        }
        ret
    }
    /// Extracts the vector represented by a blade, ignoring the e₀ component.
    pub fn to_vector(&self) -> Option<Vector> {
        (self.grade == 1).then(|| self.coefficients[1..].iter().copied().collect())
    }
    /// Returns whether the blade represents a vector.
    pub fn is_vector(&self) -> bool {
        self.grade == 1 && approx_eq(&self[Axes::E0], &0.0)
    }

    /// Constructs a blade from a hyperplane.
    pub fn from_hyperplane(ndim: u8, h: &Hyperplane) -> Self {
        let mut ret = Self::from_vector(ndim, h.normal());
        ret[Axes::E0] = -h.distance;
        ret.right_complement()
    }
    /// Extracts the hyperplane represented by a blade.
    pub fn to_hyperplane(&self) -> Option<Hyperplane> {
        if self.antigrade() != 1 {
            return None;
        }
        let b = self.left_complement();
        let normal = b.to_vector()?;
        let mag = normal.mag();
        Some(Hyperplane {
            normal: normal / mag,
            distance: -crate::util::try_div(b[Axes::E0], mag)?,
        })
    }

    /// Returns the squared [geometric norm] of the blade; i.e., the squared sum
    /// of all the coefficients of the blade.
    ///
    /// [geometric norm]:
    ///     https://rigidgeometricalgebra.org/wiki/index.php?title=Geometric_norm#Geometric_Norm
    pub fn mag2(&self) -> Float {
        self.coefficients.iter().map(|&coef| coef * coef).sum()
    }
    /// Returns the [geometric norm] of the blade; i.e., the Euclidean norm of
    /// all the coefficients of the blade.
    ///
    /// [geometric norm]:
    ///     https://rigidgeometricalgebra.org/wiki/index.php?title=Geometric_norm#Geometric_Norm
    pub fn mag(&self) -> Float {
        self.mag2().sqrt()
    }

    /// Returns the [bulk] of the blade; i.e., a blade with only the components
    /// that do not have e₀ as a factor.
    ///
    /// [bulk]:
    ///     https://rigidgeometricalgebra.org/wiki/index.php?title=Bulk_and_weight
    pub fn bulk(&self) -> Self {
        let mut bulk = Blade::zero(self.ndim, self.grade);
        for (i, &x) in self.coefficients.iter().enumerate() {
            if !self.axes_at_index(i).contains(Axes::E0) {
                bulk.coefficients[i] = x;
            }
        }
        bulk
    }
    /// Returns the [weight] of the blade; i.e., a blade with only the
    /// components that have e₀ as a factor.
    ///
    /// [weight]:
    ///     https://rigidgeometricalgebra.org/wiki/index.php?title=Bulk_and_weight
    pub fn weight(&self) -> Self {
        let mut weight = Blade::zero(self.ndim, self.grade);
        for (i, &x) in self.coefficients.iter().enumerate() {
            if self.axes_at_index(i).contains(Axes::E0) {
                weight.coefficients[i] = x;
            }
        }
        weight
    }

    /// Returns whether the blade is approximately zero.
    pub fn is_zero(&self) -> bool {
        self.coefficients.iter().all(|x| approx_eq(x, &0.0))
    }
    /// Returns whether all weight components of the blade (i.e., the components
    /// with e₀ as a factor) are approximately zero.
    pub fn weight_is_zero(&self) -> bool {
        self.terms()
            .filter(|term| term.axes.contains(Axes::E0))
            .all(|term| term.is_zero())
    }

    /// If the blade has no e₀ factor, returns the wedge product of the blade
    /// with e₀. If the blade does have some component with e₀, it is returned
    /// unmodified.
    pub fn ensure_nonzero_weight(&self) -> Blade {
        if self.weight_is_zero() {
            if let Some(product) = Blade::wedge(&Blade::from_term(self.ndim, Term::e0(1.0)), self) {
                return product;
            }
        }
        self.clone()
    }

    /// Returns the number of dimensions of the space containing the blade.
    pub fn ndim(&self) -> u8 {
        self.ndim
    }
    /// Returns the grade of the blade.
    pub fn grade(&self) -> u8 {
        self.grade
    }
    /// Returns the antigrade of the blade.
    pub fn antigrade(&self) -> u8 {
        self.ndim + 1 - self.grade // +1 because `ndim` doesn't include e₀
    }

    /// Returns the `Axes` for the `i`th coefficient.
    fn axes_at_index(&self, i: usize) -> Axes {
        Axes::from_bits_truncate(super::multivector_term_order(self.ndim, self.grade)[i])
    }
    /// Returns the index of the coefficent for `axes`.
    fn index_of(&self, axes: Axes) -> Option<usize> {
        super::multivector_term_order(self.ndim, self.grade)
            .iter()
            .position(|&it| it == axes.bits())
    }
    /// Returns an element of the blade, if it is present.
    pub fn get(&self, axes: Axes) -> Option<&Float> {
        Some(&self.coefficients[self.index_of(axes)?])
    }
    /// Returns an element of the blade, if it is present.
    pub fn get_mut(&mut self, axes: Axes) -> Option<&mut Float> {
        Some(&mut self.coefficients[self.index_of(axes)?])
    }

    /// Returns an iterator over the terms in the blade.
    pub fn terms(&self) -> impl '_ + Clone + Iterator<Item = Term> {
        self.coefficients.iter().enumerate().map(|(i, &coef)| Term {
            coef,
            axes: self.axes_at_index(i),
        })
    }
    /// Returns an iterator over the terms in the blade that are approximately
    /// nonzero.
    pub fn nonzero_terms(&self) -> impl '_ + Clone + Iterator<Item = Term> {
        self.terms().filter(|term| !term.is_zero())
    }
    /// Lifts the blade into at least `ndim`-dimensional space.
    #[must_use]
    pub fn to_ndim_at_least(&self, ndim: u8) -> Self {
        if ndim <= self.ndim {
            self.clone()
        } else {
            let mut ret = Self::zero(ndim, self.grade);
            for term in self.terms() {
                ret += term;
            }
            ret
        }
    }

    /// Returns the [exterior product] between `lhs` and `rhs`, or `None` if the
    /// result is zero because the grade of the result would exceed the number
    /// of dimensions.
    ///
    /// [exterior product]:
    ///     https://rigidgeometricalgebra.org/wiki/index.php?title=Exterior_products
    #[must_use]
    pub fn wedge(lhs: &Self, rhs: &Self) -> Option<Self> {
        let ndim = std::cmp::max(lhs.ndim, rhs.ndim);
        let grade = lhs.grade + rhs.grade;

        // +1 because `ndim` doesn't include e₀
        if grade > ndim + 1 {
            return None;
        }

        let mut ret = Self::zero(ndim, grade);
        for l in lhs.terms() {
            for r in rhs.terms() {
                ret += Term::wedge(l, r);
            }
        }
        Some(ret)
    }
    /// Returns the [exterior antiproduct] between `lhs` and `rhs`, or `None` if
    /// the result is zero because the grade of the result would exceed the
    /// number of dimensions.
    ///
    /// [exterior antiproduct]:
    ///     https://rigidgeometricalgebra.org/wiki/index.php?title=Exterior_products
    pub fn antiwedge(lhs: &Self, rhs: &Self) -> Option<Self> {
        Some(Self::wedge(&&lhs.left_complement(), &rhs.left_complement())?.right_complement())
    }
    /// Returns the [dot product] between `lhs` and `rhs`, or `None` if the
    /// arguments have different grades.
    ///
    /// [dot product]:
    ///     https://rigidgeometricalgebra.org/wiki/index.php?title=Dot_products#Dot_Product
    pub fn dot(lhs: &Self, rhs: &Self) -> Option<Float> {
        if lhs.ndim > rhs.ndim {
            Self::dot(rhs, lhs)
        } else {
            (lhs.grade == rhs.grade).then(|| {
                lhs.terms()
                    .filter(|term| !term.axes.contains(Axes::E0))
                    .map(|term| term.coef * rhs[term.axes])
                    .sum()
            })
        }
    }
    /// Returns the [dot antiproduct] between `lhs` and `rhs`, or `None` if the
    /// arguments have different grades or dimensionalities.
    ///
    /// [dot antiproduct]:
    ///     https://rigidgeometricalgebra.org/wiki/index.php?title=Dot_products#Antidot_Product
    pub fn antidot(lhs: &Self, rhs: &Self) -> Option<Term> {
        if lhs.ndim != rhs.ndim {
            return None;
        }
        let ndim = lhs.ndim;

        Some(
            Term::scalar(Self::dot(&lhs.left_complement(), &rhs.left_complement())?)
                .right_complement(ndim),
        )
    }

    /// Returns the [metric dual] of the blade.
    ///
    /// [metric dual]:
    ///     https://rigidgeometricalgebra.org/wiki/index.php?title=Duals#Dual
    #[must_use]
    pub fn dual(&self) -> Self {
        let mut ret = Self::zero(self.ndim, self.antigrade());
        for term in self.terms() {
            ret += term.dual(self.ndim);
        }
        ret
    }
    /// Returns the [metric antidual] of the blade.
    ///
    /// [metric antidual]:
    ///     https://rigidgeometricalgebra.org/wiki/index.php?title=Duals#Antidual
    #[must_use]
    pub fn antidual(&self) -> Self {
        let mut ret = Self::zero(self.ndim, self.antigrade());
        for term in self.terms() {
            ret += term.antidual(self.ndim);
        }
        ret
    }

    /// Returns the [right complement] of the blade.
    ///
    /// [right complement]:
    ///     https://rigidgeometricalgebra.org/wiki/index.php?title=Complements
    #[must_use]
    pub fn right_complement(&self) -> Self {
        let mut ret = Self::zero(self.ndim, self.antigrade());
        for term in self.terms() {
            ret += term.right_complement(self.ndim);
        }
        ret
    }
    /// Returns the [left complement] of the blade.
    ///
    /// [left complement]:
    ///     https://rigidgeometricalgebra.org/wiki/index.php?title=Complements
    #[must_use]
    pub fn left_complement(&self) -> Self {
        let mut ret = Self::zero(self.ndim, self.antigrade());
        for term in self.terms() {
            ret += term.left_complement(self.ndim);
        }
        ret
    }

    /// Returns the orthogonal projection of `self` onto `other`, or returns
    /// `None` if the operation is invalid for blades of this grade and
    /// dimensionality or if the blades are totally orthogonal.
    ///
    /// If the weight of `other` is zero, then the result would always be zero,
    /// so instead it is wedged with e₀ first.
    #[must_use]
    pub fn orthogonal_projection_to(&self, other: &Self) -> Option<Blade> {
        let ndim = common_ndim(self, other);
        let other = other.to_ndim_at_least(ndim).ensure_nonzero_weight();
        crate::util::try_div(
            Blade::antiwedge(&other, &Blade::wedge(self, &other.antidual())?)?,
            other.mag2(),
        )
    }
    /// Returns the orthogonal rejection of `self` from `other`, or returns
    /// `None` if the operation is invalid for blades of this grade and
    /// dimensionality.
    pub fn orthogonal_rejection_from(&self, other: &Self) -> Option<Blade> {
        Some(-self.orthogonal_projection_to(other)? + self)
    }

    /// Returns the 3D cross product of two blades, which are assumed to be
    /// vectors, or returns `None` if either blade is not a vector.
    #[must_use]
    pub fn cross_product_3d(lhs: &Self, rhs: &Self) -> Option<Blade> {
        if lhs.ndim == 3 && rhs.ndim == 3 && lhs.is_vector() && rhs.is_vector() {
            let bivector = Blade::wedge(&lhs, &rhs)?;
            let e0 = Blade::from_term(3, Term::e0(1.0));
            Some(Blade::wedge(&bivector, &e0)?.antidual())
        } else {
            None
        }
    }

    /// Returns an orthonormal basis for the subspace represented by a blade, or
    /// `None` if the basis is invalid.
    pub fn basis(&self) -> Vec<Vector> {
        let ndim = self.ndim;
        let rest = self.clone();
        let mut ret = vec![];
        // Set up a bitmask of remaining axes.
        let mut axes_left = ((1 as u8) << ndim) - 1;
        while ret.len() < self.grade() as usize - 1 {
            let Some((axis, v)) = crate::util::iter_ones(axes_left)
                .filter_map(|ax| {
                    let v = Vector::unit(ax as u8);
                    Some((
                        ax as u8,
                        Blade::from_vector(ndim, v).orthogonal_projection_to(&rest)?,
                    ))
                })
                .max_by_key(|(_, blade)| FloatOrd(blade.mag2()))
            else {
                break;
            };
            axes_left &= !(1 << axis);
            ret.extend(v.to_vector().and_then(|v| v.normalize()));
        }
        ret
    }
}

impl Neg for Blade {
    type Output = Blade;

    fn neg(mut self) -> Self::Output {
        for coef in self.coefficients.as_mut() {
            *coef = -*coef;
        }
        self
    }
}
impl Neg for &Blade {
    type Output = Blade;

    fn neg(self) -> Self::Output {
        -self.clone()
    }
}

impl AddAssign<Term> for Blade {
    fn add_assign(&mut self, rhs: Term) {
        self[rhs.axes] += rhs.coef;
    }
}
impl AddAssign<Option<Term>> for Blade {
    fn add_assign(&mut self, rhs: Option<Term>) {
        if let Some(r) = rhs {
            *self += r
        }
    }
}
impl AddAssign<&Blade> for Blade {
    fn add_assign(&mut self, rhs: &Blade) {
        for term in rhs.terms() {
            *self += term;
        }
    }
}
impl AddAssign<Blade> for Blade {
    fn add_assign(&mut self, rhs: Blade) {
        *self += &rhs;
    }
}

impl<T> Add<T> for Blade
where
    Blade: AddAssign<T>,
{
    type Output = Blade;

    fn add(mut self, rhs: T) -> Self::Output {
        self += rhs;
        self
    }
}

impl SubAssign<Term> for Blade {
    fn sub_assign(&mut self, rhs: Term) {
        self[rhs.axes] -= rhs.coef;
    }
}
impl SubAssign<Option<Term>> for Blade {
    fn sub_assign(&mut self, rhs: Option<Term>) {
        if let Some(r) = rhs {
            *self -= r
        }
    }
}
impl SubAssign<&Blade> for Blade {
    fn sub_assign(&mut self, rhs: &Blade) {
        for term in rhs.terms() {
            *self -= term;
        }
    }
}
impl SubAssign<Blade> for Blade {
    fn sub_assign(&mut self, rhs: Blade) {
        *self -= &rhs;
    }
}

impl<T> Sub<T> for Blade
where
    Blade: SubAssign<T>,
{
    type Output = Blade;

    fn sub(mut self, rhs: T) -> Self::Output {
        self -= rhs;
        self
    }
}
impl Mul<Float> for Blade {
    type Output = Blade;

    fn mul(mut self, rhs: Float) -> Self::Output {
        for coef in self.coefficients.as_mut() {
            *coef *= rhs;
        }
        self
    }
}
impl Mul<Float> for &Blade {
    type Output = Blade;

    fn mul(self, rhs: Float) -> Self::Output {
        self.clone() * rhs
    }
}

/// Returns the minimum number of dimensions containing two blades.
fn common_ndim(m1: &Blade, m2: &Blade) -> u8 {
    std::cmp::max(m1.ndim, m2.ndim)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_blade_orthogonal_rejection() {
        for scalar in [1.0, 0.5, 2.0, 3.5] {
            let fix = Blade::wedge(
                &Blade::from_vector(4, vector![0.0, 0.0, 0.0, 1.0]), // +W
                &Blade::from_vector(4, vector![0.0, 1.0, 1.0, 0.0]), // +Y+Z
            )
            .unwrap()
                * scalar;

            let a = vector![0.0, 0.0, 1.0, 0.0]; // +Z

            let new_a = Blade::from_vector(4, &a)
                .orthogonal_rejection_from(&fix)
                .unwrap()
                .to_vector()
                .unwrap();

            assert_approx_eq!(new_a, vector![0.0, -1.0, 1.0, 0.0] * 0.5);
        }
    }
}
