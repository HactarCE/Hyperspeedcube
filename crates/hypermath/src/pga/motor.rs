use std::fmt;
use std::hash::Hash;
use std::ops::{Add, AddAssign, Mul, MulAssign, Neg, Sub, SubAssign};

use approx_collections::{ApproxEq, ApproxEqZero, ApproxHash, Precision};

use super::{Axes, Blade, Term};
use crate::pga::blade::BivectorDecomposition;
use crate::{APPROX, Float, Hyperplane, IterWithExactSizeExt, Matrix, Point, Vector, VectorRef};

/// Sum of terms in the even or odd subalgebra of the projective geometric
/// algebra.
#[derive(Clone, PartialEq)]
pub struct Motor {
    /// Number of dimensions that the motor operates in.
    ndim: u8,
    /// Whether the motor represents a net odd number of reflections. If this is
    /// `true`, then the multivector is an element of the odd-grade PGA
    /// subalgebra; otherwise it is an element of the even-grade PGA subalgebra.
    is_reflection: bool,
    /// Coefficients of the terms of the multivector, ordered by the `Axes`
    /// values they correspond to.
    ///
    /// Terms are stored as the right complement of the actual terms so that
    /// motors can be cast into higher dimensions. Take the left complement of
    /// each term to get its original term. In practice, when using a motor to
    /// transform a multivector we take the right complement of the multivector
    /// first, then sandwich the motor with it using geometric product, then
    /// take the left complement of the result.
    coefficients: Box<[Float]>,
}

impl fmt::Debug for Motor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut ret = f.debug_struct("Motor");
        super::debug_multivector_struct_fields(&mut ret, self.terms());
        ret.finish()
    }
}

impl fmt::Display for Motor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        super::display_multivector(f, self.terms())
    }
}

impl Motor {
    /// Constructs the identity motor in `ndim` dimensions.
    pub fn ident(ndim: u8) -> Self {
        let mut ret = Self::zero(ndim, false);
        ret.set(Axes::SCALAR, 1.0);
        ret
    }
    /// Constructs a motor representing a reflection across a plane.
    ///
    /// Returns `None` if the hyperplane does not fit in `ndim` dimensions.
    pub fn plane_reflection(ndim: u8, hyperplane: &Hyperplane) -> Option<Self> {
        Self::reflection_across_blade(ndim, &Blade::from_hyperplane(ndim, hyperplane)?)
    }
    /// Constructs a motor representing a point reflection.
    ///
    /// Returns `None` if the point does not fit in `ndim` dimensions.
    pub fn point_reflection(ndim: u8, point: &Point) -> Option<Self> {
        Self::reflection_across_blade(ndim, &Blade::from_point(point))
    }
    fn reflection_across_blade(ndim: u8, blade: &Blade) -> Option<Self> {
        let mut ret = Self::zero(ndim, blade.antigrade(ndim)? % 2 == 1);
        for term in blade.terms() {
            ret += term.right_complement(ndim);
        }
        Some(ret)
    }
    /// Constructs a motor representing a reflection through the origin. Returns
    /// `None` if `vector` is zero.
    pub fn vector_reflection(vector: impl VectorRef) -> Option<Self> {
        let v = vector.normalize()?;
        Some(Self::normalized_vector_reflection(v))
    }
    /// Constructs a motor representing a reflection through the origin.
    /// `vector` **must** be normalized.
    pub fn normalized_vector_reflection(vector: impl VectorRef) -> Self {
        let mut ret = Self::zero(vector.ndim(), true);
        for (i, x) in vector.iter_nonzero() {
            ret.set(Axes::euclidean(i), x);
        }
        ret
    }
    /// Constructs a motor representing a translation by `delta`.
    pub fn translation(delta: impl VectorRef) -> Self {
        let mut ret = Self::ident(delta.ndim());
        for (i, x) in delta.iter_nonzero() {
            ret += Term {
                coef: x * -0.5,
                axes: Axes::E0 | Axes::euclidean(i),
            };
        }
        ret
    }
    /// Constructs a rotation motor (also called a "rotor") that takes one
    /// vector to another.
    pub fn rotation(from: impl VectorRef, to: impl VectorRef) -> Option<Self> {
        let from = from.normalize()?;
        let to = to.normalize()?;
        let mid = (to + &from).normalize()?;
        Some(Self::from_normalized_vector_product(from, mid))
    }
    /// Constructs a rotation from an angle in an axis-aligned plane.
    ///
    /// If the axes are the same, returns the identity.
    pub fn from_angle_in_axis_plane(a: u8, b: u8, angle: Float) -> Self {
        Self::from_angle_in_normalized_plane(Vector::unit(a), Vector::unit(b), angle)
    }
    /// Constructs a rotation motor (also called a "rotor") from one vector to
    /// another by an specific angle. `from` and `to` **must** be perpendicular
    /// unit vectors.
    pub fn from_angle_in_normalized_plane(
        a: impl VectorRef,
        b: impl VectorRef,
        angle: Float,
    ) -> Self {
        let half_angle = angle / 2.0;
        let cos = half_angle.cos();
        let sin = half_angle.sin();
        let mid = a.scale(cos) + b.scale(sin);
        Self::from_normalized_vector_product(a, mid)
    }
    /// Constructs a rotation motor (also called a "rotor") from one vector to
    /// another by twice the angle between them. `a` and `b` **must** be unit
    /// vectors.
    pub fn from_normalized_vector_product(a: impl VectorRef, b: impl VectorRef) -> Self {
        Self::normalized_vector_reflection(b) * Self::normalized_vector_reflection(a)
    }

    /// Constructs a new zero motor which can then be filled with coefficients.
    pub(crate) fn zero(ndim: u8, is_reflection: bool) -> Self {
        Self {
            ndim,
            is_reflection,
            coefficients: vec![0.0; 1 << ndim].into_boxed_slice(),
        }
    }

    /// Returns the coefficient for a term in the motor, or zero if the term
    /// does not exist.
    pub fn get(&self, axes: Axes) -> Float {
        match self.index_of(axes) {
            Some(i) => self.coefficients[i],
            None => 0.0,
        }
    }
    /// Sets the coefficient for a term in the motor.
    ///
    /// # Panics
    ///
    /// Panics in debug mode if the term is not present in the motor, either
    /// because the motor is too low-dimensional or because the motor has the
    /// wrong parity. In release mode, a warning is emitted instead and the
    /// motor is not modified.
    #[track_caller]
    fn set(&mut self, axes: Axes, value: Float) {
        if cfg!(debug_assertions) {
            self.coefficients[self.panicking_index_of(axes)] = value;
        } else {
            match self.index_of(axes) {
                Some(i) => self.coefficients[i] = value,
                None => debug_panic!("bad index {axes} into motor multivector {self}"),
            }
        }
    }

    /// Returns the number of dimensions that the motor operates in.
    pub fn ndim(&self) -> u8 {
        self.ndim
    }
    /// Returns whether the motor represents a net odd number of reflections.
    pub fn is_reflection(&self) -> bool {
        self.is_reflection
    }
    /// Returns whether the motor is the identity transformation.
    pub fn is_ident(&self) -> bool {
        self.is_equivalent_to(&Motor::ident(self.ndim))
    }

    /// Returns the `Axes` for the `i`th coefficient.
    fn axes_at_index(&self, i: usize) -> Axes {
        Self::static_axes_at_index(i, self.is_reflection)
    }
    /// Returns the `Axes` for the `i`th coefficient without needing a reference
    /// to a `Motor`.
    fn static_axes_at_index(i: usize, is_reflection: bool) -> Axes {
        let parity_correction = (i.count_ones() as u8 & 1) ^ is_reflection as u8;
        Axes::from_bits_retain((i << 1) as u8 ^ parity_correction)
    }
    /// Returns the index of the coefficient for `axes`, or `None` if the term
    /// is not present in the motor.
    pub fn index_of(&self, axes: Axes) -> Option<usize> {
        (axes.grade() & 1 == self.is_reflection as u8).then(|| axes.bits() as usize >> 1)
    }
    /// Returns the index of the coefficent for `axes`.
    ///
    /// # Panics
    ///
    /// Panics if the term does not exist in the motor due to differing parity
    /// or number of dimensions.
    #[track_caller]
    fn panicking_index_of(&self, axes: Axes) -> usize {
        self.index_of(axes)
            .expect("bad index into motor multivector")
    }

    /// Returns an iterator over the terms in the motor.
    pub fn terms(&self) -> impl '_ + Clone + Iterator<Item = Term> {
        self.coefficients.iter().enumerate().map(|(i, &coef)| Term {
            coef,
            axes: self.axes_at_index(i),
        })
    }
    /// Returns an iterator over the terms in the motor that are approximately
    /// nonzero.
    pub fn nonzero_terms(&self) -> impl '_ + Clone + Iterator<Item = Term> {
        self.terms().filter(|term| APPROX.ne_zero(term))
    }
    /// Returns the underlying array of coefficients of the motor. Avoid using
    /// these for anything other than hashing.
    pub fn coefs(&self) -> impl '_ + Clone + Iterator<Item = Float> {
        self.coefficients.iter().copied()
    }
    /// Lifts the motor into at least `ndim`-dimensional space.
    #[must_use]
    pub fn to_ndim_at_least(&self, ndim: u8) -> Self {
        if ndim <= self.ndim {
            self.clone()
        } else {
            let mut ret = Self::zero(ndim, self.is_reflection);
            for term in self.terms() {
                ret += term;
            }
            ret
        }
    }

    /// Returns whether the motor is equivalent to another motor.
    pub fn is_equivalent_to(&self, other: &Self) -> bool {
        if common_ndim_and_parity(self, other).is_none() {
            return false;
        };
        let Some(first_term_of_self) = self.nonzero_terms().next() else {
            return APPROX.eq_zero(other);
        };
        let first_term_of_other = other.get(first_term_of_self.axes);
        if APPROX.eq_zero(first_term_of_other) {
            return false;
        }
        let scale_factor = first_term_of_other / first_term_of_self.coef;
        crate::util::pad_zip(self.coefs(), other.coefs())
            .all(|(a, b)| APPROX.eq(a * scale_factor, b))
    }

    /// Returns the grade projection of the motor to a blade.
    #[must_use]
    pub fn grade_project(&self, grade: u8) -> Blade {
        let mut ret = Blade::zero_with_ndim(self.ndim, grade);
        for term in self.nonzero_terms().filter(|t| t.grade() == grade) {
            ret += term;
        }
        ret
    }

    /// Returns the motor for the reverse transformation.
    #[must_use]
    pub fn reverse(&self) -> Motor {
        let mut ret = self.clone();
        for (i, coef) in ret.coefficients.iter_mut().enumerate() {
            *coef *= self.axes_at_index(i).sign_of_reverse();
        }
        ret
    }
    /// Returns whether the motor's reverse is equivalent to itself.
    pub fn is_self_reverse(&self) -> bool {
        self.is_equivalent_to(&self.reverse())
    }
    /// Takes the corresponding power of the motor. If the exponent is negative,
    /// it uses the inverse.
    pub fn powi(&self, other: i64) -> Motor {
        // By repeated squaring
        if other == 0 {
            Self::ident(0)
        } else if other < 0 {
            self.reverse().powi(-other)
        } else {
            let init = self.powi(other >> 1);
            let squared = init.clone() * init;
            if other % 2 == 0 {
                squared
            } else {
                squared * self
            }
        }
    }

    /// Normalizes the motor so that the magnitude is `1`, or returns `None` if
    /// the motor is zero.
    pub fn normalize(&self) -> Option<Self> {
        let bulk_norm = self
            .terms()
            .filter(|term| !term.axes.contains(Axes::E0))
            .map(|term| term.coef * term.coef)
            .sum::<Float>()
            .sqrt();

        let recip = crate::util::try_recip(bulk_norm)?;
        Some(self * recip)
    }
    /// Normalizes the motor so that the magnitude is `1` and the first nonzero
    /// component is positive, or returns `None` if the motor is zero (which is
    /// invalid).
    #[must_use]
    pub fn canonicalize(&self) -> Option<Self> {
        let normalized = self.normalize()?;
        // Find the first nonzero coefficient.
        let coef = normalized.coefs().find(|x| APPROX.ne_zero(x))?;
        // Normalize so that that coefficient is zero.
        Some(if coef > 0.0 { normalized } else { -normalized })
    }
    /// Normalizes the motor so that the magnitude is `1`. If the motor is
    /// self-reverse and a rotation, then its sign is preserved; otherwise the
    /// motor is canonicalized. Returns `None` if the motor is zero (which is
    /// invalid).
    #[must_use]
    pub fn canonicalize_up_to_180(&self) -> Option<Self> {
        if !self.is_reflection() && self.is_self_reverse() {
            self.normalize()
        } else {
            self.canonicalize()
        }
    }

    /// Transforms an object using the motor.
    ///
    /// This method does not support transforming `Vector` because this is
    /// ambiguous; see [`Self::transform_vector()`] and
    /// [`Self::transform_point()`].
    pub fn transform<T: TransformByMotor>(&self, obj: &T) -> T {
        obj.transform_by(self)
    }
    /// Transforms a vector using the motor.
    ///
    /// See also [`Self::transform_point()`].
    pub fn transform_vector(&self, v: impl VectorRef) -> Vector {
        self.transform(&Blade::from_vector(v))
            .to_vector()
            .unwrap_or(Vector::EMPTY)
    }
    /// Transforms a point using the motor.
    ///
    /// See also [`Self::transform_vector()`].
    pub fn transform_point(&self, v: impl VectorRef) -> Point {
        self.transform(&Blade::from_point(&Point(v.to_vector())))
            .to_point()
            .unwrap_or(Point::ORIGIN)
    }

    /// Returns the scalar dot product between two motors.
    pub fn dot(a: &Self, b: &Self) -> Float {
        if a.is_reflection == b.is_reflection {
            // Don't bother padding the iterators, because a dot product with
            // zero will always be zero.
            std::iter::zip(a.terms(), b.terms())
                .filter(|(a, b)| !(a.axes | b.axes).contains(Axes::E0))
                .map(|(a, b)| a.coef * b.coef)
                .sum()
        } else {
            0.0
        }
    }

    /// Calls `Motor::slerp()`, using the closest of the two inputs as a backup
    /// if `slerp()` returns `None`.
    pub fn slerp_infallible(a: &Self, b: &Self, t: Float) -> Motor {
        Self::slerp(a, b, t).unwrap_or_else(|| if t < 0.5 { a } else { b }.clone())
    }
    /// Returns a spherical interpolation between `a` and `b`, or returns `None`
    /// if the motors have different number of dimensions or parity.
    pub fn slerp(a: &Self, b: &Self, t: Float) -> Option<Motor> {
        Self::slerp_non_normalized(a, b, t)?.normalize()
    }
    fn slerp_non_normalized(a: &Self, b: &Self, t: Float) -> Option<Motor> {
        // Math modified from https://docs.rs/cgmath/latest/src/cgmath/quaternion.rs.html

        let (ndim, is_reflection) = common_ndim_and_parity(a, b)?;

        let mut dot = Motor::dot(a, b);
        // Negate the second motor if that brings the rotations closer.
        let sign = if APPROX.is_neg(dot) { -1.0 } else { 1.0 };
        dot = dot.abs();

        // Stay within the domain of `acos()`.
        let robust_dot = dot.clamp(-1.0, 1.0);
        let angle = robust_dot.acos();
        let scale1 = (angle * (1.0 - t)).sin();
        let scale2 = (angle * t).sin() * sign; // Reverse the second motor if negative dot product

        Some(Motor {
            ndim,
            is_reflection,
            coefficients: crate::util::pad_zip(a.coefs(), b.coefs())
                .map(|(a, b)| a * scale1 + b * scale2)
                .collect(),
        })
    }
    /// Returns a naive linear interpolation between two motors.
    pub fn lerp_non_normalized(a: &Self, b: &Self, t: Float) -> Option<Motor> {
        let (ndim, is_reflection) = common_ndim_and_parity(a, b)?;

        Some(Motor {
            ndim,
            is_reflection,
            coefficients: crate::util::pad_zip(a.coefs(), b.coefs())
                .map(|(a, b)| crate::util::lerp(a, b, t))
                .collect(),
        })
    }

    /// Projects the motor into a lower or higher dimension without normalizing
    /// it.
    pub(crate) fn project_non_normalized(&self, ndim: u8) -> Motor {
        let mut ret = Motor::zero(ndim, self.is_reflection);
        let len = std::cmp::min(self.coefficients.len(), ret.coefficients.len());
        ret.coefficients[..len].copy_from_slice(&self.coefficients[..len]);
        ret
    }

    /// Returns the rotation matrix for a motor in Euclidean space which keeps
    /// the origin fixed.
    ///
    /// The result is undefined for any other motor.
    pub fn euclidean_rotation_matrix(&self) -> Matrix {
        Matrix::from_cols(self.euclidean_matrix_cols())
    }
    /// Returns the projective transformation matrix for a motor in
    /// `ndim`-dimensional Euclidean space.
    ///
    /// The result is undefined for any other motor.
    pub fn euclidean_projective_transformation_matrix(&self, ndim: u8) -> Matrix {
        let w = self.transform(&Point::ORIGIN).into_vector();
        let cols = self
            .euclidean_matrix_cols()
            .map(|col| col - &w)
            .chain(std::iter::once(Vector::unit(ndim) + &w))
            .with_exact_size(ndim as usize + 1);
        Matrix::from_cols(cols)
    }
    fn euclidean_matrix_cols(&self) -> impl '_ + ExactSizeIterator<Item = Vector> {
        (0..self.ndim).map(|i| self.transform(&Vector::unit(i)))
    }

    /// Returns the tangent of the logarithm of a motor. Returns None if the
    /// motor is a reflection. <https://arxiv.org/abs/2107.03771>
    pub fn tan_bivector_log(&self) -> Option<Blade> {
        if self.is_reflection {
            return None;
        }
        Some(self.grade_project(2) / self.grade_project(0).to_scalar()?)
    }

    /// Returns the logarithm of a motor. Returns None if the motor is a
    /// reflection or decomposition fails. <https://arxiv.org/abs/2107.03771>
    pub(crate) fn log_as_decomposition(&self) -> Option<BivectorDecomposition> {
        let tan_log = self.tan_bivector_log()?;
        tan_log.decompose_bivector()?.atan()
    }

    /// Returns the logarithm of a motor. Returns None if the motor is a
    /// reflection or the computation fails. <https://arxiv.org/abs/2107.03771>
    pub fn log(&self) -> Option<Blade> {
        let tan_log = self.tan_bivector_log()?;

        tan_log.atan()
    }

    /// Takes the corresponding power of the motor.
    pub fn powf(&self, other: Float) -> Option<Motor> {
        let mut decomposition = self.log_as_decomposition()?;
        decomposition *= other;
        decomposition.exp()
    }
}

impl AddAssign<Term> for Motor {
    fn add_assign(&mut self, rhs: Term) {
        self.set(rhs.axes, self.get(rhs.axes) + rhs.coef);
    }
}
impl Add<Term> for Motor {
    type Output = Motor;
    fn add(self, rhs: Term) -> Motor {
        let mut ret = self.clone();
        ret += rhs;
        ret
    }
}
impl AddAssign<Blade> for Motor {
    fn add_assign(&mut self, rhs: Blade) {
        for term in rhs.terms() {
            *self += term;
        }
    }
}
impl Add<Blade> for Motor {
    type Output = Motor;
    fn add(self, rhs: Blade) -> Motor {
        let mut ret = self.clone();
        ret += rhs;
        ret
    }
}
impl AddAssign<Motor> for Motor {
    fn add_assign(&mut self, rhs: Motor) {
        for term in rhs.terms() {
            *self += term;
        }
    }
}
impl Add<Motor> for Motor {
    type Output = Motor;
    fn add(self, rhs: Motor) -> Motor {
        let mut ret = self.clone();
        ret += rhs;
        ret
    }
}
impl SubAssign<Term> for Motor {
    fn sub_assign(&mut self, rhs: Term) {
        self.set(rhs.axes, self.get(rhs.axes) - rhs.coef);
    }
}
impl Sub<Term> for Motor {
    type Output = Motor;
    fn sub(self, rhs: Term) -> Motor {
        let mut ret = self.clone();
        ret -= rhs;
        ret
    }
}
impl SubAssign<Blade> for Motor {
    fn sub_assign(&mut self, rhs: Blade) {
        for term in rhs.terms() {
            *self -= term;
        }
    }
}
impl Sub<Blade> for Motor {
    type Output = Motor;
    fn sub(self, rhs: Blade) -> Motor {
        let mut ret = self.clone();
        ret -= rhs;
        ret
    }
}
impl SubAssign<Motor> for Motor {
    fn sub_assign(&mut self, rhs: Motor) {
        for term in rhs.terms() {
            *self -= term;
        }
    }
}
impl Sub<Motor> for Motor {
    type Output = Motor;
    fn sub(self, rhs: Motor) -> Motor {
        let mut ret = self.clone();
        ret -= rhs;
        ret
    }
}

impl Mul<&Motor> for &Motor {
    type Output = Motor;

    fn mul(self, rhs: &Motor) -> Self::Output {
        let mut ret = Motor::zero(
            std::cmp::max(self.ndim, rhs.ndim),
            self.is_reflection ^ rhs.is_reflection,
        );
        for l in self.terms() {
            for r in rhs.terms() {
                if let Some(product) = l * r {
                    ret += product;
                }
            }
        }
        ret
    }
}
impl Mul<Float> for Motor {
    type Output = Motor;

    fn mul(mut self, rhs: Float) -> Self::Output {
        self *= rhs;
        self
    }
}
impl Mul<Float> for &Motor {
    type Output = Motor;

    fn mul(self, rhs: Float) -> Self::Output {
        self.clone() * rhs
    }
}

impl_forward_bin_ops_to_ref! {
    impl Mul for Motor { fn mul() }
}

impl MulAssign<Motor> for Motor {
    fn mul_assign(&mut self, rhs: Self) {
        *self = &*self * rhs;
    }
}
impl MulAssign<&Motor> for Motor {
    fn mul_assign(&mut self, rhs: &Self) {
        *self = &*self * rhs;
    }
}
impl MulAssign<Float> for Motor {
    fn mul_assign(&mut self, rhs: Float) {
        for coef in &mut self.coefficients[..] {
            *coef *= rhs;
        }
    }
}

impl Neg for Motor {
    type Output = Motor;

    /// Negates the coefficients of the motor. It still represents the same
    /// transformation.
    fn neg(mut self) -> Self::Output {
        for coef in &mut self.coefficients[..] {
            *coef = -*coef;
        }
        self
    }
}
impl Neg for &Motor {
    type Output = Motor;

    /// Negates the coefficients of the motor. It still represents the same
    /// transformation.
    fn neg(self) -> Self::Output {
        -self.clone()
    }
}

impl ApproxEq for Motor {
    fn approx_eq(&self, other: &Self, prec: Precision) -> bool {
        self.is_reflection == other.is_reflection
            && crate::util::pad_zip(self.coefs(), other.coefs()).all(|(a, b)| prec.eq(a, b))
    }
}
impl ApproxEqZero for Motor {
    /// Returns whether the motor has all zero terms (and therefore does not
    /// represent a valid transformation).
    fn approx_eq_zero(&self, prec: Precision) -> bool {
        self.coefs().all(|x| prec.eq_zero(x))
    }
}
impl ApproxHash for Motor {
    fn intern_floats<F: FnMut(&mut f64)>(&mut self, f: &mut F) {
        self.coefficients.intern_floats(f);
    }

    fn interned_eq(&self, other: &Self) -> bool {
        self.is_reflection == other.is_reflection
            && self.coefficients.interned_eq(&other.coefficients)
    }

    fn interned_hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.is_reflection.hash(state);
        self.coefficients.interned_hash(state);
    }
}

/// Trait for things that can be transformed by a [`Motor`].
pub trait TransformByMotor {
    /// Transform the object by the motor `m`.
    fn transform_by(&self, m: &Motor) -> Self;
}

impl TransformByMotor for Vector {
    fn transform_by(&self, m: &Motor) -> Self {
        m.transform_vector(self)
    }
}

impl TransformByMotor for Point {
    fn transform_by(&self, m: &Motor) -> Self {
        m.transform_point(self.as_vector())
    }
}

impl TransformByMotor for Hyperplane {
    fn transform_by(&self, m: &Motor) -> Self {
        let ndim = std::cmp::max(m.ndim, self.normal().ndim());
        let ret = Blade::from_hyperplane(ndim, self)
            .expect("error constructing hyperplane")
            .transform_by(m)
            .to_hyperplane(ndim)
            .unwrap_or_else(|| {
                debug_panic!("error transforming hyperplane {self} by {m:?}");
                Hyperplane {
                    normal: vector![],
                    distance: 0.0,
                }
            });
        // Transforming a blade reflects its orientation (a clockwise arrow on
        // the hyperplane will now point counterclockwise) but we want to
        // preserve its inside/outside. This case handles that correctly.
        if m.is_reflection() && ndim % 2 == 1 {
            ret.flip()
        } else {
            ret
        }
    }
}

impl TransformByMotor for Blade {
    fn transform_by(&self, m: &Motor) -> Self {
        let ndim = std::cmp::max(m.ndim, self.ndim());
        let mut result = Blade::zero_with_ndim(ndim, self.grade());
        for (u, l, r) in
            itertools::iproduct!(self.nonzero_terms(), m.nonzero_terms(), m.nonzero_terms())
        {
            let u = u.right_complement(ndim);
            if let Some(product) = triple_geometric_product([l, u, r.reverse()]) {
                let product = product.left_complement(ndim);
                if product.grade() == self.grade() {
                    result[product.axes] += product.coef;
                }
            }
        }
        if m.is_reflection() && ndim.is_multiple_of(2) {
            -result
        } else {
            result
        }
    }
}

impl TransformByMotor for Motor {
    fn transform_by(&self, m: &Motor) -> Self {
        let ndim = std::cmp::max(m.ndim, self.ndim);
        let mut result = Motor::zero(ndim, self.is_reflection);
        // Don't take the complement of `m` because it's already stored as the
        // right complement.
        for (u, l, r) in
            itertools::iproduct!(self.nonzero_terms(), m.nonzero_terms(), m.nonzero_terms())
        {
            if let Some(product) = triple_geometric_product([l, u, r.reverse()]) {
                result += product;
            }
        }
        result
    }
}

impl<T: TransformByMotor> TransformByMotor for Vec<T> {
    fn transform_by(&self, m: &Motor) -> Self {
        self.iter().map(|obj| m.transform(obj)).collect()
    }
}

impl<T: TransformByMotor> TransformByMotor for Option<T> {
    fn transform_by(&self, m: &Motor) -> Self {
        self.as_ref().map(|inner| inner.transform_by(m))
    }
}

macro_rules! impl_for_tuples {
    ($impl_macro:ident) => {
        $impl_macro!(T0; 0);
        $impl_macro!(T0, T1; 0, 1);
        $impl_macro!(T0, T1, T2; 0, 1, 2);
        $impl_macro!(T0, T1, T2, T3; 0, 1, 2, 3);
        $impl_macro!(T0, T1, T2, T3, T4; 0, 1, 2, 3, 4);
        $impl_macro!(T0, T1, T2, T3, T4, T5; 0, 1, 2, 3, 4, 5);
        $impl_macro!(T0, T1, T2, T3, T4, T5, T6; 0, 1, 2, 3, 4, 5, 6);
        $impl_macro!(T0, T1, T2, T3, T4, T5, T6, T7; 0, 1, 2, 3, 4, 5, 6, 7);
        $impl_macro!(T0, T1, T2, T3, T4, T5, T6, T7, T8; 0, 1, 2, 3, 4, 5, 6, 7, 8);
        $impl_macro!(T0, T1, T2, T3, T4, T5, T6, T7, T8, T9; 0, 1, 2, 3, 4, 5, 6, 7, 8, 9);
    };
}
macro_rules! impl_transform_by_motor_for_tuple {
    ($($generic_param:ident),+; $($index:tt),+) => {
        impl<$($generic_param: TransformByMotor,)+> TransformByMotor for ($($generic_param,)+) {
            fn transform_by(&self, m: &Motor) -> Self {
                ($(self.$index.transform_by(m),)+)
            }
        }
    };
}
impl_for_tuples!(impl_transform_by_motor_for_tuple);

fn triple_geometric_product(terms: [Term; 3]) -> Option<Term> {
    let [a, b, c] = terms;
    let tmp = Term::geometric_product(a, b)?;
    Term::geometric_product(tmp, c)
}

/// Returns the minimum number of dimensions containing two motors.
fn common_ndim(m1: &Motor, m2: &Motor) -> u8 {
    std::cmp::max(m1.ndim, m2.ndim)
}

/// Returns the minimum number of dimensions containing two motors and their
/// common parity, or returns `None` if they have different parities.
fn common_ndim_and_parity(m1: &Motor, m2: &Motor) -> Option<(u8, bool)> {
    (m1.is_reflection == m2.is_reflection).then(|| (common_ndim(m1, m2), m1.is_reflection))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test a formula for extracting the second fixed axis of a rotation in 4D,
    /// given the rotation and one known fixed axis.
    #[test]
    fn test_4d_extract_second_fixed_axis() {
        let rot = Motor::from_angle_in_axis_plane(0, 1, 0.2);

        let ax1 = vector![0.0, 0.0, 1.0];
        let ax2 = Blade::wedge(
            &Blade::wedge(&Blade::origin(), &Blade::from_vector(ax1)).unwrap(),
            &rot.grade_project(2),
        )
        .unwrap()
        .antidual(4)
        .unwrap();

        // ax2 should be a unit vector along the W axis.
        assert_eq!(1, ax2.grade());
        assert!(APPROX.ne_zero(&ax2));
        let wedge = Blade::wedge(&ax2, &Blade::from_vector(vector![0.0, 0.0, 0.0, 1.0]));
        assert!(wedge.is_some_and(|b| APPROX.eq_zero(b)));
    }

    #[test]
    fn test_motor_powf() {
        let motors = vec![
            Motor::rotation([1.0, 2.0, 3.0, 4.0, 5.0], [1.0, 2.0, 3.0, 4.0, -5.0]).unwrap(),
            Motor::rotation([1.0, 0.0], [0.0, 1.0]).unwrap(),
            Motor::from_angle_in_axis_plane(0, 1, std::f64::consts::PI),
        ];
        for motor in motors {
            dbg!(&motor);
            assert_approx_eq!(motor.log().unwrap().exp().unwrap(), motor);
            assert_approx_eq!(motor.powf(1.0).unwrap(), motor);
            let motor1 = motor.powf(0.3).unwrap();
            let motor2 = motor.powf(0.7).unwrap();
            assert_approx_eq!(motor1 * motor2, motor);
        }
    }

    #[test]
    fn test_transform_vector() {
        for motor_ndim in 2..=6 {
            let rot = Motor::from_normalized_vector_product(
                vector![1.0],
                vector![1.0, 1.0].normalize().unwrap(),
            )
            .to_ndim_at_least(motor_ndim);
            let refl = Motor::vector_reflection(vector![1.0])
                .unwrap()
                .to_ndim_at_least(motor_ndim);

            let v = vector![1.0];
            assert_approx_eq!(rot.transform(&v), vector![0.0, 1.0]);
            assert_approx_eq!(refl.transform(&v), vector![-1.0]);
        }
    }

    #[test]
    fn test_transform_point() {
        for motor_ndim in 2..=6 {
            let rot = Motor::from_normalized_vector_product(vector![1.0], vector![1.0, 1.0])
                .to_ndim_at_least(motor_ndim);
            let refl = Motor::vector_reflection(vector![1.0])
                .unwrap()
                .to_ndim_at_least(motor_ndim);

            let p = point![1.0];
            assert_approx_eq!(rot.transform(&p), point![0.0, 1.0]);
            assert_approx_eq!(refl.transform(&p), point![-1.0]);
        }
    }

    #[test]
    fn test_geometric_antiproduct() {
        let ndim = 3;
        for b in (0..16).map(Axes::from_bits_truncate).map(Term::unit) {
            for a in (0..16).map(Axes::from_bits_truncate).map(Term::unit) {
                let antiantiproduct =
                    Term::geometric_antiproduct(a, b, ndim).map(|t| t.right_complement(ndim));
                let product =
                    Term::geometric_product(a.right_complement(ndim), b.right_complement(ndim));
                if let (Some(antiantiproduct), Some(product)) = (antiantiproduct, product) {
                    assert_approx_eq!(antiantiproduct, product);
                } else {
                    assert_eq!(antiantiproduct, product);
                }
            }
        }
    }

    #[test]
    fn test_reflect_hyperplane() {
        let refl = Motor::vector_reflection(vector![1.0]).unwrap();
        for ax in 0..7 {
            let init = Hyperplane::from_pole(Vector::unit(ax)).unwrap();
            let expected = if ax == 0 {
                Hyperplane::from_pole(vector![-1.0]).unwrap()
            } else {
                init.clone()
            };
            assert_approx_eq!(expected, refl.transform(&init));
        }
    }
}
