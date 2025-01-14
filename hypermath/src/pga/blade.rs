use std::fmt;
use std::ops::{Add, AddAssign, Div, Index, IndexMut, Mul, Neg, Sub, SubAssign};

use float_ord::FloatOrd;
use itertools::Itertools;

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
    /// Extracts the scalar represented by a blade.
    pub fn to_scalar(&self) -> Option<Float> {
        (self.grade == 0).then(|| self.coefficients[0])
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

    /// Returns an iterator over all terms in the [bulk] of the blade; i.e., the
    /// components that do not have e₀ as a factor.
    ///
    /// [bulk]:
    ///     https://rigidgeometricalgebra.org/wiki/index.php?title=Bulk_and_weight
    fn bulk_terms(&self) -> impl '_ + Iterator<Item = Term> {
        self.terms().filter(|term| !term.axes.contains(Axes::E0))
    }
    /// Returns an iterator over all terms in the [weight] of the blade; i.e.,
    /// the components that have e₀ as a factor.
    ///
    /// [weight]:
    ///     https://rigidgeometricalgebra.org/wiki/index.php?title=Weight_and_weight
    fn weight_terms(&self) -> impl '_ + Iterator<Item = Term> {
        self.terms().filter(|term| term.axes.contains(Axes::E0))
    }

    /// Returns the [bulk] of the blade; i.e., a blade with only the components
    /// that do not have e₀ as a factor.
    ///
    /// [bulk]:
    ///     https://rigidgeometricalgebra.org/wiki/index.php?title=Bulk_and_weight
    pub fn bulk(&self) -> Self {
        let mut bulk = Blade::zero(self.ndim, self.grade);
        for term in self.bulk_terms() {
            bulk += term;
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
        for term in self.weight_terms() {
            weight += term;
        }
        weight
    }

    /// Returns the [bulk norm] of the blade; i.e., the magnitude of the terms
    /// that do not have e₀ as a factor.
    ///
    /// [bulk norm]:
    ///     https://rigidgeometricalgebra.org/wiki/index.php?title=Geometric_norm#Bulk_Norm
    pub fn bulk_norm(&self) -> Float {
        self.bulk_terms()
            .map(|term| term.coef * term.coef)
            .sum::<Float>()
            .sqrt()
    }
    /// Returns the [weight norm] of the blade; i.e., the magnitude of the terms
    /// that have e₀ as a factor.
    ///
    /// [weight norm]:
    ///     https://rigidgeometricalgebra.org/wiki/index.php?title=Geometric_norm#Weight_Norm
    pub fn weight_norm(&self) -> Float {
        self.weight_terms()
            .map(|term| term.coef * term.coef)
            .sum::<Float>()
            .sqrt()
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

    /// Returns the projection of the product of the blades to the specified grade.
    pub fn product_grade(lhs: &Self, rhs: &Self, grade: u8) -> Self {
        let ndim = std::cmp::max(lhs.ndim, rhs.ndim);

        let mut ret = Self::zero(ndim, grade);
        for l in lhs.terms() {
            for r in rhs.terms() {
                let Some(term) = Term::geometric_product(l, r) else {
                    continue;
                };
                if term.grade() == grade {
                    ret += term;
                }
            }
        }
        ret
    }
    /// Returns the projection of the product of the blades to the specified grade.
    pub fn multi_product_grade(blades: impl IntoIterator<Item = Blade>, grade: u8) -> Self {
        let blades = blades.into_iter().collect_vec();
        let ndim = blades.iter().map(|b| b.ndim).max().unwrap_or(0);

        let mut ret = Self::zero(ndim, grade);

        'a: for terms in blades
            .into_iter()
            .map(|b| b.terms().collect_vec())
            .multi_cartesian_product()
        {
            let mut term = Term::scalar(1.0);
            for t in terms {
                if let Some(tt) = Term::geometric_product(term, t) {
                    term = tt
                } else {
                    continue 'a;
                }
            }
            if term.grade() == grade {
                ret += term;
            }
        }

        ret
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
        Some(Self::wedge(&lhs.left_complement(), &rhs.left_complement())?.right_complement())
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
            let bivector = Blade::wedge(lhs, rhs)?;
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
        let mut covered = self.right_complement(); // left vs. right doesn't matter
        let mut ret = vec![];
        // Set up a bitmask of remaining axes.
        let mut axes_left = (1_u8 << ndim) - 1;
        while ret.len() < self.grade() as usize - 1 {
            let Some((axis, v)) = crate::util::iter_ones(axes_left)
                .filter_map(|ax| {
                    let v = Vector::unit(ax as u8);
                    Some((
                        ax as u8,
                        Blade::from_vector(ndim, v).orthogonal_rejection_from(&covered)?,
                    ))
                })
                .max_by_key(|(_, blade)| FloatOrd(blade.mag2()))
            else {
                break;
            };
            axes_left &= !(1 << axis);
            covered = Blade::wedge(&covered, &v).unwrap_or(covered);
            ret.extend(v.to_vector().and_then(|v| v.normalize()));
        }
        ret
    }

    /// Decompose bivector into a sum of commuting simple bivectors, or
    /// `None` if the input is not a bivector. Some terms may contain multiple bivectors of the same magnitude.
    /// https://arxiv.org/abs/2107.03771
    fn decompose_bivector(&self) -> Option<Vec<(u8, Self)>> {
        if self.grade != 2 {
            return None;
        }

        if self.ndim < 4 {
            // All bivectors are simple
            Some(vec![(1, self.clone())])
        } else if self.ndim < 6 {
            // Section 6.1
            let coeff1 = Self::product_grade(self, self, 0).to_scalar()?;
            let wedge = Self::product_grade(self, self, 4);
            let wedge2 = Self::product_grade(&wedge, &wedge, 0).to_scalar()?; // square of wedge
            let discrim = coeff1.powi(2) - wedge2;
            if approx_eq(&discrim, &0.0) {
                Some(vec![(2, self.clone())])
            } else if discrim < 0.0 {
                None // impossible, i think
            } else {
                let root1 = (coeff1 + discrim.sqrt()) / 2.0;
                let root2 = (coeff1 - discrim.sqrt()) / 2.0;
                // Compute (root1 + wedge * 0.5) / self
                // 1/self = (self * m(self^2)) / (self^2 * m(self^2)) where m(grade0) = grade0 and m(grade4) = -grade4
                // it is of grade 2
                let selfi_2 = (self * coeff1 - Self::product_grade(self, &wedge, 2)) / discrim;
                let selfi_4 = (-Self::product_grade(self, &wedge, 4)) / discrim; // this term should be 0 in 4d
                let b1 = selfi_2.clone() * root1
                    + (Self::product_grade(&wedge, &selfi_2, 2)
                        + Self::product_grade(&wedge, &selfi_4, 2))
                        * 0.5;
                let b2 = selfi_2.clone() * root2
                    + (Self::product_grade(&wedge, &selfi_2, 2)
                        + Self::product_grade(&wedge, &selfi_4, 2))
                        * 0.5;
                Some(vec![(1, b1), (1, b2)])
            }
        } else if self.ndim < 8 {
            return None; // TODO: scary
        } else {
            return None; // TODO: blow up if this happens?
        }
    }

    /// Returns the exponential of a bivector, or
    /// `None` if the input is not a bivector.
    pub fn exp(&self) -> Option<crate::pga::Motor> {
        use crate::pga::Motor;
        let decomposition = self.decompose_bivector()?;

        let mut ret = Motor::ident(self.ndim);
        // the bivs commute
        for (mult, biv) in decomposition {
            let norm2 = Float::max(0.0, Self::dot(&biv, &biv)?) / (mult as Float);
            let norm = norm2.sqrt();
            let cos = norm.cos(); // we want the sign flip of dot
            let sin = (1.0 - cos.powi(2)).sqrt();
            let biv1 = biv / norm;
            let motor;
            if mult == 1 {
                motor = (Motor::ident(self.ndim) * cos) + (biv1 * sin);
            } else {
                // if biv = b1 + b2 + ..., then biv2 = b1 b2 + ...
                let biv2 = Self::product_grade(&biv1, &biv1, 4) / 2.0;
                if mult == 2 {
                    motor =
                        (Motor::ident(self.ndim) * cos * cos) + biv1 * cos * sin + biv2 * sin * sin;
                } else {
                    // if biv = b1 + b2 + b3 + ..., then biv3 = b1 b2 b3 + ...
                    let biv3 =
                        Self::multi_product_grade([biv1.clone(), biv1.clone(), biv1.clone()], 6)
                            / 6.0;
                    if mult == 3 {
                        motor = (Motor::ident(self.ndim) * cos * cos * cos)
                            + biv1 * cos * cos * sin
                            + biv2 * cos * sin * sin
                            + biv3 * sin * sin * sin;
                    } else {
                        return None; // TODO: blow up if this happens? (only dimension 8+)
                    }
                }
            }
            ret *= motor;
        }

        Some(ret)
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
            *self += r;
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
            *self -= r;
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
impl Div<Float> for Blade {
    type Output = Blade;

    fn div(mut self, rhs: Float) -> Self::Output {
        for coef in self.coefficients.as_mut() {
            *coef /= rhs;
        }
        self
    }
}
impl Div<Float> for &Blade {
    type Output = Blade;

    fn div(self, rhs: Float) -> Self::Output {
        self.clone() / rhs
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

    #[test]
    fn test_bivector_decomposition_s() {
        let b1o = Blade::wedge(
            &Blade::from_vector(4, vector![1.0, 0.0, 0.0, 0.0]),
            &Blade::from_vector(4, vector![0.0, 1.0, 0.0, 0.0]),
        )
        .unwrap();
        let b2o = Blade::wedge(
            &Blade::from_vector(4, vector![0.0, 0.0, 1.0, 0.0]),
            &Blade::from_vector(4, vector![0.0, 0.0, 0.0, 1.0]),
        )
        .unwrap()
            * 2.0;
        let bivector = b1o.clone() + b2o.clone();

        let out = bivector.decompose_bivector().unwrap();
        assert!(out.len() == 2);
        let b1 = &out[0].1;
        let b2 = &out[1].1;
        assert_approx_eq!(b1.clone() + b2.clone(), bivector);
        assert_approx_eq!(Blade::dot(&b1, &b2).unwrap(), 0.0);
    }

    #[test]
    fn test_bivector_decomposition() {
        let b1o = Blade::wedge(
            &Blade::from_vector(5, vector![1.0, 0.0, 0.0, 0.0, 6.0]),
            &Blade::from_vector(5, vector![-1.0, 0.0, 0.0, 0.0, 6.0]),
        )
        .unwrap();
        let b2o = Blade::wedge(
            &Blade::from_vector(5, vector![0.0, 2.0, 3.0, 5.0, 0.0]),
            &Blade::from_vector(5, vector![0.0, -2.0, 3.0, 5.0, 0.0]),
        )
        .unwrap();
        let bivector = b1o.clone() + b2o.clone();

        dbg!(&bivector, Blade::dot(&b1o, &b1o), Blade::dot(&b2o, &b2o));
        let out = bivector.decompose_bivector().unwrap();
        assert!(out.len() == 2);
        let b1 = &out[0].1;
        let b2 = &out[1].1;
        assert_approx_eq!(b1.clone() + b2.clone(), bivector);
        assert_approx_eq!(Blade::dot(&b1, &b2).unwrap(), 0.0);
    }

    #[test]
    fn test_exp_rational() {
        let b1o = Blade::wedge(
            &Blade::from_vector(4, vector![1.0, 0.0, 0.0, 0.0]),
            &Blade::from_vector(4, vector![0.0, 1.0, 0.0, 0.0]),
        )
        .unwrap();
        let b2o = Blade::wedge(
            &Blade::from_vector(4, vector![0.0, 0.0, 1.0, 0.0]),
            &Blade::from_vector(4, vector![0.0, 0.0, 0.0, 1.0]),
        )
        .unwrap();
        let bivector = b1o.clone() + b2o.clone();
        assert_approx_eq!(
            (bivector.clone() * std::f64::consts::PI).exp().unwrap(),
            crate::pga::Motor::ident(4)
        );
        let clifford_45 = (bivector * std::f64::consts::PI / 4.0).exp().unwrap();
        dbg!(&clifford_45);
        assert_approx_eq!(
            clifford_45.powi(4).canonicalize_up_to_180().unwrap(),
            crate::pga::Motor::ident(4)
        )
    }
}
