use std::fmt;
use std::hash::Hash;
use std::ops::{Add, AddAssign, Div, Index, IndexMut, Mul, MulAssign, Neg, Sub, SubAssign};

use approx_collections::{ApproxEq, ApproxEqZero, ApproxHash, Precision};
use float_ord::FloatOrd;
use itertools::Itertools;

use super::{Axes, Term};
use crate::util::PI;
use crate::{APPROX, Float, Hyperplane, Point, Vector, VectorRef};

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

impl ApproxEq for Blade {
    fn approx_eq(&self, other: &Self, prec: Precision) -> bool {
        self.grade == other.grade
            && crate::util::pad_zip(
                self.coefficients.iter().copied(),
                other.coefficients.iter().copied(),
            )
            .all(|(l, r)| prec.eq(l, r))
    }
}
impl ApproxEqZero for Blade {
    fn approx_eq_zero(&self, prec: Precision) -> bool {
        self.coefficients.iter().all(|x| prec.eq_zero(x))
    }
}
impl ApproxHash for Blade {
    fn intern_floats<F: FnMut(&mut f64)>(&mut self, f: &mut F) {
        self.coefficients.intern_floats(f);
    }

    fn interned_eq(&self, other: &Self) -> bool {
        self.ndim == other.ndim
            && self.grade == other.grade
            && self.coefficients.interned_eq(&other.coefficients)
    }

    fn interned_hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.ndim.hash(state);
        self.grade.hash(state);
        self.coefficients.interned_hash(state);
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
    /// Constructs a new zero blade of grade `grade`.
    pub fn zero(grade: u8) -> Self {
        Self::zero_with_ndim(0, grade)
    }
    /// Constructs a new zero blade of grade `grade` in `ndim` dimensions.
    pub fn zero_with_ndim(ndim: u8, grade: u8) -> Self {
        let len = super::multivector_term_order(ndim, grade).len();
        Self {
            ndim,
            grade,
            coefficients: vec![0.0; len].into_boxed_slice(),
        }
    }
    /// Constructs a unit blade of grade 0.
    pub fn one() -> Self {
        Self::scalar(1.0)
    }
    /// Constructs a blade of grade 0.
    pub fn scalar(value: Float) -> Self {
        Self {
            ndim: 0,
            grade: 0,
            coefficients: vec![value].into_boxed_slice(),
        }
    }
    /// Constructs a blade from a single term.
    pub fn from_term(term: Term) -> Self {
        let mut ret = Self::zero(term.grade());
        ret[term.axes] = term.coef;
        ret
    }

    /// Constructs a blade representing the point at the origin.
    pub fn origin() -> Self {
        let mut ret = Self::zero(1);
        ret[Axes::E0] = 1.0;
        ret
    }
    /// Constructs a blade from a point.
    pub fn from_point(p: &Point) -> Self {
        let mut ret = Self::from_vector(p.as_vector());
        ret[Axes::E0] = 1.0;
        ret
    }
    /// Extracts the point represented by a blade.
    pub fn to_point(&self) -> Option<Point> {
        crate::util::try_div(self.to_vector()?, self[Axes::E0]).map(Point)
    }
    /// Returns whether the blade represents a point.
    pub fn is_point(&self) -> bool {
        self.grade == 1 && APPROX.ne_zero(self[Axes::E0])
    }

    /// Constructs a blade from a vector.
    pub fn from_vector(v: impl VectorRef) -> Self {
        Self::from_vector_with_ndim(v.ndim(), v)
    }
    /// Constructs a blade from a vector in `ndim` dimensions.
    pub fn from_vector_with_ndim(ndim: u8, v: impl VectorRef) -> Self {
        let mut ret = Self::zero_with_ndim(ndim, 1);
        for (i, x) in v.iter_ndim(v.ndim()).enumerate() {
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
        self.grade == 1 && APPROX.eq_zero(self[Axes::E0])
    }
    /// Extracts the scalar represented by a blade.
    pub fn to_scalar(&self) -> Option<Float> {
        (self.grade == 0).then(|| self.coefficients[0])
    }

    /// Constructs a blade from a hyperplane.
    ///
    /// Returns `None` if `h` does not fit in `ndim` dimensions.
    pub fn from_hyperplane(ndim: u8, h: &Hyperplane) -> Option<Self> {
        let mut ret = Self::from_vector_with_ndim(ndim, h.normal());
        ret[Axes::E0] = -h.distance;
        ret.right_complement(ndim)
    }
    /// Extracts the hyperplane represented by a blade.
    pub fn to_hyperplane(&self, ndim: u8) -> Option<Hyperplane> {
        if self.antigrade(ndim) != Some(1) {
            return None;
        }
        let b = self.left_complement(ndim)?;
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
        let mut bulk = Blade::zero_with_ndim(self.ndim, self.grade);
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
        let mut weight = Blade::zero_with_ndim(self.ndim, self.grade);
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

    /// Normalizes the blade relative to [`Self::weight_norm()`].
    ///
    /// If the weight norm is zero, the blade is returned unmodified.
    #[must_use]
    pub fn weight_normalize(&self) -> Self {
        crate::util::try_div(self, self.weight_norm()).unwrap_or_else(|| self.clone())
    }

    /// Returns whether all weight components of the blade (i.e., the components
    /// with e₀ as a factor) are approximately zero.
    pub fn weight_is_zero(&self) -> bool {
        self.terms()
            .filter(|term| term.axes.contains(Axes::E0))
            .all(|term| APPROX.eq_zero(term))
    }

    /// If the blade has no e₀ factor, returns the wedge product of the blade
    /// with e₀. If the blade does have some component with e₀, it is returned
    /// unmodified.
    pub fn ensure_nonzero_weight(&self) -> Blade {
        if self.weight_is_zero()
            && let Some(product) = Blade::wedge(&Blade::from_term(Term::e0(1.0)), self)
        {
            return product;
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
    ///
    /// Returns `None` if the blade requires more than `ndim` dimensions.
    pub fn antigrade(&self, ndim: u8) -> Option<u8> {
        (ndim + 1).checked_sub(self.grade) // +1 because `ndim` doesn't include e₀
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
    /// Returns an element of the blade, if it is present. If `axes` has the
    /// correct grade but requires a greater number of dimensions, then the
    /// blade is extended to the required number of dimensions.
    pub fn get_mut(&mut self, axes: Axes) -> Option<&mut Float> {
        self.ensure_ndim_at_least(axes.min_ndim());
        Some(&mut self.coefficients[self.index_of(axes)?])
    }

    /// Returns an iterator over the terms in the blade.
    pub fn terms(&self) -> impl '_ + Clone + Iterator<Item = Term> {
        self.coefficients.iter().enumerate().map(|(i, &coef)| Term {
            coef,
            axes: self.axes_at_index(i),
        })
    }
    /// Returns an iterator over the terms in the blade that fit within
    /// `ndim`-dimensional space.
    pub fn terms_in_ndim(&self, ndim: u8) -> impl '_ + Clone + Iterator<Item = Term> {
        let antiscalar = Axes::antiscalar(ndim);
        self.terms()
            .filter(move |term| antiscalar.contains(term.axes))
    }
    /// Returns an iterator over the terms in the blade that are approximately
    /// nonzero.
    pub fn nonzero_terms(&self) -> impl '_ + Clone + Iterator<Item = Term> {
        self.terms().filter(|term| APPROX.ne_zero(term))
    }
    /// Returns an iterator over the terms in the blade that are approximately
    /// nonzero and fit within `ndim`-dimensional space.
    pub fn nonzero_terms_in_ndim(&self, ndim: u8) -> impl '_ + Clone + Iterator<Item = Term> {
        self.terms_in_ndim(ndim).filter(|term| APPROX.ne_zero(term))
    }
    /// Lifts the blade into at least `ndim`-dimensional space.
    ///
    /// Returns a new blade.
    #[must_use]
    pub fn to_ndim_at_least(&self, ndim: u8) -> Self {
        if ndim <= self.ndim {
            return self.clone();
        }
        let len = super::multivector_term_order(ndim, self.grade).len();
        let coefficients = self
            .coefficients
            .iter()
            .copied()
            .pad_using(len, |_| 0.0)
            .collect();
        Self {
            ndim,
            grade: self.grade,
            coefficients,
        }
    }
    /// Lifts the blade into at least `ndim`-dimensional space.
    ///
    /// Modifies the blade in-place.
    pub fn ensure_ndim_at_least(&mut self, ndim: u8) {
        if self.ndim < ndim {
            *self = self.to_ndim_at_least(ndim);
        }
    }

    /// Returns the projection of the product of the blades to the specified
    /// grade.
    pub fn product_grade(lhs: &Self, rhs: &Self, grade: u8) -> Self {
        let ndim = std::cmp::max(lhs.ndim, rhs.ndim);

        let mut ret = Self::zero_with_ndim(ndim, grade);
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

    /// Returns the projection of the product of the blades to the grade 0. May
    /// differ from dot by a sign.
    pub fn product_scalar(lhs: &Self, rhs: &Self) -> Float {
        let mut ret = 0.0;
        for l in lhs.terms() {
            for r in rhs.terms() {
                let Some(term) = Term::geometric_product(l, r) else {
                    continue;
                };
                if term.grade() == 0 {
                    ret += term.coef;
                }
            }
        }
        ret
    }
    /// Returns the projection of the product of the blades to the specified
    /// grade.
    pub fn multi_product_grade(blades: impl IntoIterator<Item = Self>, grade: u8) -> Self {
        let blades = blades.into_iter().collect_vec();
        let ndim = blades.iter().map(|b| b.ndim).max().unwrap_or(0);

        let mut ret = Self::zero_with_ndim(ndim, grade);

        'a: for terms in blades.iter().map(|b| b.terms()).multi_cartesian_product() {
            let mut term = Term::scalar(1.0);
            for t in terms {
                if let Some(tt) = Term::geometric_product(term, t) {
                    term = tt;
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

    /// Returns the projection of the `pow`-th power of the blade to the
    /// specified grade.
    pub fn power_grade(blade: &Self, pow: u32, grade: u8) -> Self {
        let blades = vec![blade.clone(); pow as usize];
        Self::multi_product_grade(blades, grade)
    }

    /// Returns the [exterior product] between `lhs` and `rhs`, or `None` if the
    /// result is zero because the grade of the result would exceed the number
    /// of dimensions.
    ///
    /// TODO: revisit
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

        let mut ret = Self::zero_with_ndim(ndim, grade);
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
    /// TODO: revisit
    ///
    /// Returns `None` if the blade requires more than `ndim` dimensions.
    ///
    /// [exterior antiproduct]:
    ///     https://rigidgeometricalgebra.org/wiki/index.php?title=Exterior_products
    pub fn antiwedge(ndim: u8, lhs: &Self, rhs: &Self) -> Option<Self> {
        Self::wedge(&lhs.left_complement(ndim)?, &rhs.left_complement(ndim)?)?
            .right_complement(ndim)
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
    /// arguments have different dimensionalities or cannot be represented in
    /// `ndim`-dimensional space.
    ///
    /// [dot antiproduct]:
    ///     https://rigidgeometricalgebra.org/wiki/index.php?title=Dot_products#Antidot_Product
    pub fn antidot(ndim: u8, lhs: &Self, rhs: &Self) -> Option<Term> {
        Some(
            Term::scalar(Self::dot(
                &lhs.left_complement(ndim)?,
                &rhs.left_complement(ndim)?,
            )?)
            .right_complement(ndim),
        )
    }

    /// Returns the [metric dual] of the term in `ndim`-dimensional space, or
    /// `None` if it is zero. The blade is implicitly projected to
    /// `ndim`-dimensional space.
    ///
    /// Returns `None` if the blade requires more than `ndim` dimensions.
    ///
    /// [metric dual]:
    ///     https://rigidgeometricalgebra.org/wiki/index.php?title=Duals#Dual
    #[must_use]
    pub fn dual(&self, ndim: u8) -> Option<Self> {
        let mut ret = Self::zero_with_ndim(ndim, self.antigrade(ndim)?);
        for term in self.terms_in_ndim(ndim) {
            ret += term.dual(ndim);
        }
        Some(ret)
    }
    /// Returns the [metric antidual] of the blade. The blade is implicitly
    /// projected to `ndim`-dimensional space.
    ///
    /// Returns `None` if the blade requires more than `ndim` dimensions.
    ///
    /// [metric antidual]:
    ///     https://rigidgeometricalgebra.org/wiki/index.php?title=Duals#Antidual
    #[must_use]
    pub fn antidual(&self, ndim: u8) -> Option<Self> {
        let mut ret = Self::zero_with_ndim(ndim, self.antigrade(ndim)?);
        for term in self.terms_in_ndim(ndim) {
            ret += term.antidual(ndim);
        }
        Some(ret)
    }

    /// Returns the [right complement] of the blade.
    ///
    /// Returns `None` if the blade requires more than `ndim` dimensions.
    ///
    /// [right complement]:
    ///     https://rigidgeometricalgebra.org/wiki/index.php?title=Complements
    #[must_use]
    pub fn right_complement(&self, ndim: u8) -> Option<Self> {
        let mut ret = Self::zero_with_ndim(ndim, self.antigrade(ndim)?);
        for term in self.terms_in_ndim(ndim) {
            ret += term.right_complement(ndim);
        }
        Some(ret)
    }
    /// Returns the [left complement] of the blade. The blade is implicitly
    /// projected to `ndim`-dimensional space.
    ///
    /// Returns `None` if the blade requires more than `ndim` dimensions.
    ///
    /// [left complement]:
    ///     https://rigidgeometricalgebra.org/wiki/index.php?title=Complements
    #[must_use]
    pub fn left_complement(&self, ndim: u8) -> Option<Self> {
        let mut ret = Self::zero_with_ndim(ndim, self.antigrade(ndim)?);
        for term in self.terms_in_ndim(ndim) {
            ret += term.left_complement(ndim);
        }
        Some(ret)
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
            Blade::antiwedge(ndim, &other, &Blade::wedge(self, &other.antidual(ndim)?)?)?,
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
        if lhs.is_vector() && rhs.is_vector() {
            let bivector = Blade::wedge(lhs, rhs)?;
            let e0 = Blade::from_term(Term::e0(1.0));
            Some(Blade::wedge(&bivector, &e0)?.antidual(3)?)
        } else {
            None
        }
    }

    /// Returns an orthonormal basis for the subspace represented by a blade, or
    /// `None` if the basis is invalid.
    pub fn basis(&self) -> Vec<Vector> {
        let ndim = self.ndim;
        let mut covered = self
            .right_complement(ndim) // left vs. right doesn't matter
            .expect("right complement failed");
        let mut ret = vec![];
        // Set up a bitmask of remaining axes.
        let mut axes_left = (1_u8 << ndim) - 1;
        while ret.len() < self.grade() as usize - 1 {
            let Some((axis, v)) = crate::util::iter_ones(axes_left)
                .filter_map(|ax| {
                    let v = Vector::unit(ax as u8);
                    Some((
                        ax as u8,
                        Blade::from_vector(v).orthogonal_rejection_from(&covered)?,
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

    /// Decompose bivector into a sum of commuting simple bivectors, or `None`
    /// if the input is not a bivector. Some terms may contain multiple
    /// bivectors of the same magnitude. <https://arxiv.org/abs/2107.03771>
    pub(crate) fn decompose_bivector(&self) -> Option<BivectorDecomposition> {
        if self.grade != 2 {
            return None;
        }

        let decomposition = if self.ndim < 4 {
            // All bivectors are simple
            Some(vec![(1, self.clone())])
        } else if self.ndim < 6 {
            // Section 6.1
            let coeff1 = Self::product_scalar(self, self);
            let wedge = Self::product_grade(self, self, 4);
            let wedge2 = Self::product_scalar(&wedge, &wedge); // square of wedge
            let discrim = coeff1.powi(2) - wedge2;
            if APPROX.eq_zero(discrim) {
                Some(vec![(2, self.clone())])
            } else if discrim < 0.0 {
                None // impossible, i think
            } else {
                let root1 = (coeff1 + discrim.sqrt()) / 2.0;
                let root2 = (coeff1 - discrim.sqrt()) / 2.0;
                // Compute (root1 + wedge * 0.5) / self
                // 1/self = (self * m(self^2)) / (self^2 * m(self^2)) where m(grade0) = grade0
                // and m(grade4) = -grade4 it is of grade 2
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
            // Section 6.5
            let coeff2 = -Self::product_scalar(self, self);
            let wedge = Self::product_grade(self, self, 4);
            let coeff1 = Self::product_scalar(&wedge, &wedge) / 4.0;
            let wedge3 = Self::power_grade(self, 3, 6);
            let coeff0 = -Self::product_scalar(&wedge3, &wedge3) / 36.0;

            let roots = {
                // Depress the cubic
                // https://en.wikipedia.org/wiki/Cubic_equation#Depressed_cubic
                let p = (3.0 * coeff1 - coeff2 * coeff2) / 3.0;
                let q = (2.0 * coeff2.powi(3) - 9.0 * coeff2 * coeff1 + 27.0 * coeff0) / 27.0;

                // There should be 3 real roots
                if APPROX.eq_zero(p) {
                    // In this case q should also be zero, so the roots are all the same
                    [-coeff2 / 3.0; 3]
                } else {
                    // https://en.wikipedia.org/wiki/Cubic_equation#Trigonometric_solution_for_three_real_roots
                    [0.0, 1.0, 2.0].map(|k| {
                        2.0 * (-p / 3.0).sqrt()
                            * ((1.5 * q / p * (-3.0 / p).sqrt()).acos() / 3.0 - 2.0 * PI * k / 3.0)
                                .cos()
                            - coeff2 / 3.0
                    })
                }
            };

            let unique_root: Option<Option<usize>>;
            if APPROX.eq(roots[0], roots[1]) {
                if APPROX.eq(roots[0], roots[2]) {
                    unique_root = None;
                } else {
                    unique_root = Some(Some(2));
                }
            } else if APPROX.eq(roots[0], roots[2]) {
                unique_root = Some(Some(1));
            } else if APPROX.eq(roots[1], roots[2]) {
                unique_root = Some(Some(0));
            } else {
                unique_root = Some(None);
            }

            'dec: {
                let Some(unique_root) = unique_root else {
                    // All three roots are equal
                    break 'dec Some(vec![(3, self.clone())]);
                };

                let single_roots = match unique_root {
                    // Root i is distinct from the other two, which are equal
                    Some(i) => vec![roots[i]],
                    // All three roots are different
                    None => roots.to_vec(),
                };

                let mut decomposition = Vec::new();
                for root in single_roots {
                    // Take the inverse of root + wedge / 2.0
                    let den = Multivector04 {
                        grade0: root,
                        grade4: wedge.clone() / 2.0,
                    };
                    let deni = den.recip()?;

                    // Multiply self * root + wedge3 / 6.0 by that inverse
                    let biv = self * root * deni.grade0
                        + Self::product_grade(self, &deni.grade4, 2) * root
                        + Self::product_grade(&wedge3, &deni.grade4, 2) / 6.0;
                    decomposition.push((1, biv));
                }

                if unique_root.is_some() {
                    decomposition.push((2, self.clone() - decomposition[0].1.clone()));
                }

                Some(decomposition)
            }
        } else {
            log::error!("cannot decompose 8D+ bivector");
            return None;
        };

        let ret = decomposition.map(|decomposition| BivectorDecomposition {
            ndim: self.ndim,
            decomposition,
        });

        ret.map(|r| r.remove_zeros())
    }

    /// Returns the exponential of a bivector, or `None` if the input is not a
    /// bivector.
    pub fn exp(&self) -> Option<crate::pga::Motor> {
        let decomposition = self.decompose_bivector()?;
        decomposition.exp()
    }

    /// Returns the arctangent of a bivector, or `None` if the input is not a
    /// bivector. <https://arxiv.org/abs/2107.03771>
    pub fn atan(&self) -> Option<Self> {
        Some(self.decompose_bivector()?.atan()?.to_bivector())
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
    #[track_caller]
    fn add_assign(&mut self, rhs: Term) {
        self[rhs.axes] += rhs.coef;
    }
}
impl AddAssign<Option<Term>> for Blade {
    #[track_caller]
    fn add_assign(&mut self, rhs: Option<Term>) {
        if let Some(r) = rhs {
            *self += r;
        }
    }
}
impl AddAssign<&Blade> for Blade {
    #[track_caller]
    fn add_assign(&mut self, rhs: &Blade) {
        for term in rhs.terms() {
            *self += term;
        }
    }
}
impl AddAssign<Blade> for Blade {
    #[track_caller]
    fn add_assign(&mut self, rhs: Blade) {
        *self += &rhs;
    }
}

impl<T> Add<T> for Blade
where
    Blade: AddAssign<T>,
{
    type Output = Blade;

    #[track_caller]
    fn add(mut self, rhs: T) -> Self::Output {
        self += rhs;
        self
    }
}

impl SubAssign<Term> for Blade {
    #[track_caller]
    fn sub_assign(&mut self, rhs: Term) {
        self[rhs.axes] -= rhs.coef;
    }
}
impl SubAssign<Option<Term>> for Blade {
    #[track_caller]
    fn sub_assign(&mut self, rhs: Option<Term>) {
        if let Some(r) = rhs {
            *self -= r;
        }
    }
}
impl SubAssign<&Blade> for Blade {
    #[track_caller]
    fn sub_assign(&mut self, rhs: &Blade) {
        for term in rhs.terms() {
            *self -= term;
        }
    }
}
impl SubAssign<Blade> for Blade {
    #[track_caller]
    fn sub_assign(&mut self, rhs: Blade) {
        *self -= &rhs;
    }
}

impl<T> Sub<T> for Blade
where
    Blade: SubAssign<T>,
{
    type Output = Blade;

    #[track_caller]
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

#[derive(Debug)]
pub(crate) struct BivectorDecomposition {
    ndim: u8,
    decomposition: Vec<(u8, Blade)>,
}

impl BivectorDecomposition {
    fn remove_zeros(mut self) -> Self {
        self.decomposition
            .retain(|(_mult, biv)| APPROX.ne_zero(biv));
        self
    }

    pub(crate) fn exp(&self) -> Option<crate::pga::Motor> {
        use crate::pga::Motor;

        let mut ret = Motor::ident(self.ndim);
        // the bivs commute
        for (mult, biv) in &self.decomposition {
            let norm2 = Float::max(0.0, Blade::dot(biv, biv)?) / (*mult as Float);
            let norm = norm2.sqrt();
            let cos = norm.cos(); // we want the sign flip of dot
            let sin = (1.0 - cos.powi(2)).sqrt();
            let biv1 = biv / norm;
            let motor;
            if APPROX.eq_zero(biv) {
                motor = Motor::ident(self.ndim);
            } else if *mult == 1 {
                motor = (Motor::ident(self.ndim) * cos) + (biv1 * sin);
            } else {
                // if biv = b1 + b2 + ..., then biv2 = b1 b2 + ...
                let biv2 = Blade::product_grade(&biv1, &biv1, 4) / 2.0;
                if *mult == 2 {
                    motor =
                        (Motor::ident(self.ndim) * cos * cos) + biv1 * cos * sin + biv2 * sin * sin;
                } else {
                    // if biv = b1 + b2 + b3 + ..., then biv3 = b1 b2 b3 + ...
                    let biv3 =
                        Blade::multi_product_grade([biv1.clone(), biv1.clone(), biv1.clone()], 6)
                            / 6.0;
                    if *mult == 3 {
                        motor = (Motor::ident(self.ndim) * cos * cos * cos)
                            + biv1 * cos * cos * sin
                            + biv2 * cos * sin * sin
                            + biv3 * sin * sin * sin;
                    } else {
                        log::error!("cannot exponentiate 8D+ bivector");
                        return None;
                    }
                }
            }
            ret *= motor;
        }

        Some(ret)
    }

    fn to_bivector(&self) -> Blade {
        let mut ret = Blade::zero_with_ndim(self.ndim, 2);
        for (_mult, biv) in &self.decomposition {
            ret += biv;
        }
        ret
    }

    pub(crate) fn atan(&self) -> Option<Self> {
        let mut decomposition = Vec::new();
        for (mult, biv) in &self.decomposition {
            let norm2 = Float::max(0.0, Blade::dot(biv, biv)?) / (*mult as Float);
            let norm = norm2.sqrt();
            let biv1 = biv / norm;
            decomposition.push((*mult, biv1 * norm.atan()));
        }
        Some(BivectorDecomposition {
            ndim: self.ndim,
            decomposition,
        })
    }
}

impl MulAssign<Float> for BivectorDecomposition {
    fn mul_assign(&mut self, rhs: Float) {
        for (_mult, biv) in &mut self.decomposition {
            *biv = biv.clone() * rhs;
        }
    }
}

/// Multivector with grade 0 and 4 components. Should be closed under
/// multiplication.
#[derive(Debug, Clone, PartialEq)]
struct Multivector04 {
    grade0: Float,
    grade4: Blade,
}

impl Multivector04 {
    fn ndim(&self) -> u8 {
        self.grade4.ndim
    }

    fn zero(ndim: u8) -> Self {
        Self {
            grade0: 0.0,
            grade4: Blade::zero_with_ndim(ndim, 4),
        }
    }

    fn one(ndim: u8) -> Self {
        Self {
            grade0: 1.0,
            grade4: Blade::zero_with_ndim(ndim, 4),
        }
    }

    fn dot(&self, rhs: &Multivector04) -> Float {
        self.grade0 * rhs.grade0 + Blade::product_scalar(&self.grade4, &rhs.grade4)
    }

    fn recip(&self) -> Option<Self> {
        let mut pows = vec![Self::one(self.ndim())];
        for i in 0..4 {
            pows.push(pows[i as usize].clone() * self);
        }

        for n in 1..=4 {
            let matrix =
                crate::Matrix::from_fn(n, |i, j| pows[i as usize + 1].dot(&pows[j as usize + 1]));

            // matrix is only 4x4 so inverting it is fine
            if APPROX.eq_zero(matrix.determinant()) {
                continue;
            }

            let target_vec = Vector(pows[1..].iter().map(|p| p.grade0).collect());
            let Some(minv) = matrix.inverse() else {
                continue;
            };
            // Just do matrix multiplication here I don't care
            let mut test_recip = Self::zero(self.ndim());
            // This is where we divide by self
            for i in 0..n {
                let result_vec_entry = VectorRef::dot(&minv.row(i), target_vec.clone());
                test_recip = test_recip + &(pows[i as usize].clone() * result_vec_entry);
            }

            if APPROX.eq(test_recip.clone() * self, Self::one(self.ndim())) {
                return Some(test_recip);
            }
        }

        None
    }
}

impl Mul<&Multivector04> for Multivector04 {
    type Output = Multivector04;

    fn mul(self, rhs: &Multivector04) -> Self::Output {
        // assum
        let grade0 = self.dot(rhs);
        let grade4 = self.grade4.clone() * rhs.grade0
            + rhs.grade4.clone() * self.grade0
            + Blade::product_grade(&self.grade4, &rhs.grade4, 4);
        Self { grade0, grade4 }
    }
}
impl Mul<Float> for Multivector04 {
    type Output = Multivector04;

    fn mul(self, rhs: Float) -> Self::Output {
        // assum
        Self {
            grade0: self.grade0 * rhs,
            grade4: self.grade4 * rhs,
        }
    }
}
impl Add<&Multivector04> for Multivector04 {
    type Output = Multivector04;

    fn add(self, rhs: &Multivector04) -> Self::Output {
        Self {
            grade0: self.grade0 + rhs.grade0,
            grade4: self.grade4 + rhs.grade4.clone(),
        }
    }
}
impl ApproxEq for Multivector04 {
    fn approx_eq(&self, other: &Self, prec: Precision) -> bool {
        prec.eq(self.grade0, other.grade0) && prec.eq(&self.grade4, &other.grade4)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_blade_orthogonal_rejection() {
        for scalar in [1.0, 0.5, 2.0, 3.5] {
            let fix = Blade::wedge(
                &Blade::from_vector(vector![0.0, 0.0, 0.0, 1.0]), // +W
                &Blade::from_vector(vector![0.0, 1.0, 1.0, 0.0]), // +Y+Z
            )
            .unwrap()
                * scalar;

            let a = vector![0.0, 0.0, 1.0]; // +Z

            let new_a = Blade::from_vector(&a)
                .orthogonal_rejection_from(&fix)
                .unwrap()
                .to_vector()
                .unwrap();

            assert_approx_eq!(new_a, vector![0.0, -1.0, 1.0, 0.0] * 0.5);
        }
    }

    fn test_bivector_decomposition_specific(vs: Vec<[Vector; 2]>) {
        let bso = vs
            .iter()
            .map(|[v1, v2]| Blade::wedge(&Blade::from_vector(v1), &Blade::from_vector(v2)).unwrap())
            .collect_vec();

        let bivector = bso.iter().fold(Blade::zero(2), Add::add);

        let out = bivector.decompose_bivector().unwrap();
        assert_approx_eq!(
            out.decomposition
                .iter()
                .map(|b| b.1.clone())
                .fold(Blade::zero(2), Add::add),
            bivector
        );
        dbg!(&out);
        for i in 0..out.decomposition.len() {
            for j in 0..i {
                assert_approx_eq!(
                    Blade::dot(&out.decomposition[i].1, &out.decomposition[j].1).unwrap(),
                    0.0
                );
            }
        }
    }

    #[test]
    fn test_bivector_decomposition_4() {
        test_bivector_decomposition_specific(vec![
            [vector![1.0, 0.0, 0.0, 0.0], vector![0.0, 1.0, 0.0, 0.0]],
            [vector![0.0, 0.0, 1.0, 0.0], vector![0.0, 0.0, 0.0, 1.0]],
        ]);
    }
    #[test]
    fn test_bivector_decomposition_4b() {
        test_bivector_decomposition_specific(vec![
            [
                vector![1.0, 0.0, 3.0, 0.0, 6.0],
                vector![-1.0, 0.0, 3.0, 0.0, 6.0],
            ],
            [
                vector![0.0, 2.0, 0.0, 5.0, 0.0],
                vector![0.0, -2.0, 0.0, 5.0, 0.0],
            ],
        ]);
    }
    #[test]
    fn test_bivector_decomposition_6_2() {
        test_bivector_decomposition_specific(vec![
            [
                vector![1.0, 0.0, 0.0, 0.0, 0.0, 0.0],
                vector![0.0, 1.0, 0.0, 0.0, 0.0, 0.0],
            ],
            [
                vector![0.0, 0.0, 1.0, 0.0, 0.0, 0.0],
                vector![0.0, 0.0, 0.0, 1.0, 0.0, 0.0],
            ],
        ]);
    }
    #[test]
    fn test_bivector_decomposition_6_2b() {
        test_bivector_decomposition_specific(vec![
            [
                vector![1.0, 0.0, 0.0, 0.0, 0.0, 0.0],
                vector![0.0, 1.0, 0.0, 0.0, 0.0, 0.0],
            ],
            [
                vector![0.0, 0.0, 2.0, 0.0, 0.0, 0.0],
                vector![0.0, 0.0, 0.0, 1.0, 0.0, 0.0],
            ],
        ]);
    }
    #[test]
    fn test_bivector_decomposition_6_3() {
        test_bivector_decomposition_specific(vec![
            [
                vector![1.0, 0.0, 0.0, 0.0, 0.0, 0.0],
                vector![0.0, 1.0, 0.0, 0.0, 0.0, 0.0],
            ],
            [
                vector![0.0, 0.0, 2.0, 0.0, 0.0, 0.0],
                vector![0.0, 0.0, 0.0, 1.0, 0.0, 0.0],
            ],
            [
                vector![0.0, 0.0, 0.0, 0.0, 3.0, 0.0],
                vector![0.0, 0.0, 0.0, 0.0, 0.0, 1.0],
            ],
        ]);
    }

    #[test]
    fn test_exp_rational() {
        let b1o = Blade::wedge(
            &Blade::from_vector(vector![1.0, 0.0, 0.0, 0.0]),
            &Blade::from_vector(vector![0.0, 1.0, 0.0, 0.0]),
        )
        .unwrap();
        let b2o = Blade::wedge(
            &Blade::from_vector(vector![0.0, 0.0, 1.0, 0.0]),
            &Blade::from_vector(vector![0.0, 0.0, 0.0, 1.0]),
        )
        .unwrap();
        let bivector = b1o.clone() + b2o.clone();
        assert_approx_eq!(
            (bivector.clone() * PI).exp().unwrap(),
            crate::pga::Motor::ident(4)
        );
        let clifford_45 = (bivector * PI / 4.0).exp().unwrap();
        dbg!(&clifford_45);
        assert_approx_eq!(
            clifford_45.powi(4).canonicalize_up_to_180().unwrap(),
            crate::pga::Motor::ident(4)
        )
    }
}
