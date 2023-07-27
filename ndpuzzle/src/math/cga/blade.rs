//! Multivectors of a single grade, which are called blades.

use anyhow::{bail, Result};
use itertools::Itertools;
use std::fmt;
use std::ops::{BitXor, Mul, MulAssign, Neg, Shl};

use super::{AsMultivector, Axes, Multivector, Term};
use crate::math::*;

/// Multivector of a single grade, which can represent a point, line, plane,
/// circle, sphere, etc., or zero. An N-blade is a blade where each term has N
/// axes, and can be written as the product of N orthogonal vectors.
///
/// When a blade is used to represent an object such as a point, plane, or
/// sphere, it is either in OPNS (outer product null space) form or IPNS (inner
/// product null space) form. OPNS means that the object's outer (wedge) product
/// with a point is zero iff the point is on the object, and IPNS means that the
/// object's inner (dot) product with a point is zero iff the point is on the
/// object. OPNS form is easier to construct by wedging several points together,
/// which gives the object tangent to all those points. Some operations are
/// easier to do using OPNS form and some are easier to do using IPNS form, so
/// this struct has methods to convert between them.
///
/// # Panics
///
/// Many of the methods on this type will panic if passed a blade of the wrong
/// grade (zero is okay).
#[derive(Debug, Clone, PartialEq)]
pub struct Blade(pub(super) Multivector);

impl fmt::Display for Blade {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

impl AbsDiffEq for Blade {
    type Epsilon = Float;

    fn default_epsilon() -> Self::Epsilon {
        Multivector::default_epsilon()
    }

    fn abs_diff_eq(&self, other: &Self, epsilon: Self::Epsilon) -> bool {
        self.0.abs_diff_eq(&other.0, epsilon)
    }
}

impl From<Term> for Blade {
    fn from(term: Term) -> Self {
        Blade(Multivector::from(term))
    }
}

impl From<Blade> for Multivector {
    fn from(value: Blade) -> Self {
        value.0
    }
}
impl TryFrom<Multivector> for Blade {
    type Error = MismatchedGrade;

    fn try_from(multivector: Multivector) -> std::result::Result<Self, Self::Error> {
        if multivector
            .terms()
            .iter()
            .map(|term| term.grade())
            .all_equal()
        {
            Ok(Blade(multivector))
        } else {
            Err(MismatchedGrade)
        }
    }
}
pub struct MismatchedGrade;

impl AsMultivector for Blade {
    fn mv(&self) -> &Multivector {
        &self.0
    }
    fn into_mv(self) -> Multivector {
        self.0
    }
}

/// Negation of a blade.
impl<'a> Neg for &'a Blade {
    type Output = Blade;

    fn neg(self) -> Self::Output {
        Blade(-&self.0)
    }
}
impl Neg for Blade {
    type Output = Blade;

    fn neg(self) -> Self::Output {
        Blade(-self.0)
    }
}

/// Scaling a blade by a number.
impl<'a> Mul<Float> for &'a Blade {
    type Output = Blade;

    fn mul(self, rhs: Float) -> Self::Output {
        Blade(&self.0 * rhs)
    }
}
impl Mul<Float> for Blade {
    type Output = Blade;

    fn mul(self, rhs: Float) -> Self::Output {
        Blade(self.0 * rhs)
    }
}
impl MulAssign<Float> for Blade {
    fn mul_assign(&mut self, rhs: Float) {
        self.0 *= rhs;
    }
}

/// Outer product of a blade and a term.
///
/// See https://w.wiki/6L8p
impl<'a> BitXor<Term> for &'a Blade {
    type Output = Blade;

    fn bitxor(self, rhs: Term) -> Self::Output {
        Blade(&self.0 ^ rhs)
    }
}
impl BitXor<Term> for Blade {
    type Output = Blade;

    fn bitxor(self, rhs: Term) -> Self::Output {
        Blade(self.0 ^ rhs)
    }
}
/// Outer product of a term and a blade.
///
/// See https://w.wiki/6L8p
impl<'a> BitXor<&'a Blade> for Term {
    type Output = Blade;

    fn bitxor(self, rhs: &'a Blade) -> Self::Output {
        Blade(self ^ &rhs.0)
    }
}
impl BitXor<Blade> for Term {
    type Output = Blade;

    fn bitxor(self, rhs: Blade) -> Self::Output {
        Blade(self ^ rhs.0)
    }
}

/// Left contraction of a term and a blade.
///
/// See https://youtu.be/oVyBbJl6xvo?t=180s for an intuitive explanation.
impl<'a> Shl<&'a Blade> for Term {
    type Output = Blade;

    fn shl(self, rhs: &'a Blade) -> Self::Output {
        Blade(self << &rhs.0)
    }
}
impl Shl<Blade> for Term {
    type Output = Blade;

    fn shl(self, rhs: Blade) -> Self::Output {
        Blade(self << rhs.0)
    }
}

/// Outer product of two blades.
///
/// Intuitively, this constructs the object passing through both objects. For
/// example, the outer product of a point and a circle is the sphere tangent to
/// both. If the objects already intersect, then the result is zero.
impl<'a> BitXor for &'a Blade {
    type Output = Blade;

    fn bitxor(self, rhs: Self) -> Self::Output {
        Blade(&self.0 ^ &rhs.0)
    }
}
impl_forward_bin_ops_to_ref! {
    impl BitXor for Blade { fn bitxor() }
}

/// Left contraction of two blades.
///
/// See https://youtu.be/oVyBbJl6xvo?t=180s for an intuitive explanation.
impl<'a> Shl for &'a Blade {
    type Output = Blade;

    fn shl(self, rhs: Self) -> Self::Output {
        Blade(&self.0 << &rhs.0)
    }
}
impl_forward_bin_ops_to_ref! {
    impl Shl for Blade { fn shl() }
}

impl Blade {
    /// Zero blade.
    ///
    /// This represents a degenerate object, which is not the same as the point
    /// at the origin.
    pub const ZERO: Self = Blade(Multivector::ZERO);

    /// Null vector representing the point at the origin.
    ///
    /// See https://w.wiki/6L8q
    pub const NO: Self = Blade(Multivector::NO);
    /// Null vector representing the point at infinity.
    ///
    /// See https://w.wiki/6L8q
    pub const NI: Self = Blade(Multivector::NI);

    /// Constructs a scalar blade.
    pub fn scalar(s: Float) -> Self {
        Blade(Multivector::scalar(s))
    }
    /// Constructs the pseudoscalar for a particular number of dimensions. This
    /// is the OPNS blade representing the whole space.
    pub fn pseudoscalar(ndim: u8) -> Self {
        Blade(Multivector::from(Term::pseudoscalar(ndim)))
    }
    /// Constructs the inverse pseudoscalar for a particular number of
    /// dimensions.
    pub fn inverse_pseudoscalar(ndim: u8) -> Self {
        Blade(Multivector::from(Term::inverse_pseudoscalar(ndim)))
    }
    /// Constructs the blade representing a vector.
    pub fn vector(v: impl VectorRef) -> Self {
        Blade(Multivector::from(v))
    }
    /// Constructs the normalized OPNS blade representing a point.
    pub fn point(p: impl ToConformalPoint) -> Self {
        p.to_normalized_1blade()
    }
    /// Constructs the OPNS blade representing a point.
    ///
    /// See https://w.wiki/6L8o
    fn vector_to_point(p: impl VectorRef) -> Self {
        // p + NO + 1/2 * NI * ||p||
        Blade(Multivector::from(&p) + Multivector::NO + Multivector::NI * 0.5 * p.mag2())
    }
    /// Constructs the OPNS blade representing the pair of `p` and the point at
    /// infinity, which is called a "flat point."
    pub fn flat_point(p: impl VectorRef) -> Self {
        Blade::point(p) ^ Blade::NI
    }
    /// Constructs an IPNS blade representing a hypersphere.
    ///
    /// If the radius is negative, constructs an "inside-out" sphere. This is
    /// kind of a hack but it's convenient.
    pub fn ipns_sphere(center: impl VectorRef, radius: Float) -> Self {
        let mv = Blade::point(center).mv() - Multivector::NI * radius * radius * 0.5;
        Blade(mv * radius.signum())
    }
    /// Constructs an IPNS blade representing an imaginary hypersphere.
    pub fn ipns_imaginary_sphere(center: impl VectorRef, radius: Float) -> Self {
        let mv = Blade::point(center).mv() + Multivector::NI * radius * radius * 0.5;
        Blade(mv * radius.signum())
    }
    /// Constructs an IPNS blade representing a hyperplane. If `distance` is
    /// positive, then the origin is considered "inside."
    pub fn ipns_plane(normal: impl VectorRef, distance: Float) -> Self {
        match normal.normalize() {
            // Negate so that the origin is "inside."
            Some(normal) => Blade(-Multivector::from(normal) - Multivector::NI * distance),
            None => Blade::ZERO,
        }
    }

    /// Normalizes an OPNS point to +No, if the No component is nonzero, or +Ni
    /// otherwise.
    #[must_use]
    pub fn normalize_point(&self) -> Self {
        let no = self.no();
        if is_approx_nonzero(&no) {
            return self * no.recip();
        }
        let ni = self.ni();
        if is_approx_nonzero(&ni) {
            return self * ni.recip();
        }
        self.clone()
    }

    /// Converts an OPNS blade to an IPNS blade, given the number of dimensions
    /// of the whole space.
    #[must_use]
    pub fn opns_to_ipns(&self, ndim: u8) -> Self {
        self << Blade::inverse_pseudoscalar(ndim)
    }
    /// Converts an IPNS blade to an OPNS blade, given the number of dimensions
    /// of the whole space.
    #[must_use]
    pub fn ipns_to_opns(&self, ndim: u8) -> Self {
        self << Blade::pseudoscalar(ndim)
    }

    /// Converts an OPNS blade to an IPNS blade, given an OPNS blade
    /// representing the space it inhabits. Returns zero if `opns_space` cannot
    /// be inverted.
    #[must_use]
    pub fn opns_to_ipns_in_space(&self, opns_space: &Blade) -> Self {
        self << opns_space.inverse().unwrap_or(Blade::ZERO)
    }
    /// Converts an IPNS blade to an OPNS blade, given an OPNS blade
    /// representing the space it inhabits.
    #[must_use]
    pub fn ipns_to_opns_in_space(&self, opns_space: &Blade) -> Self {
        self << opns_space
    }

    /// Returns the reverse blade, which has all the same terms but with the
    /// axes reversed (which in practice just means some signs are flipped).
    #[must_use]
    pub fn reverse(&self) -> Self {
        Blade(self.mv().reverse())
    }
    /// Returns the multiplicative inverse of the blade.
    #[must_use]
    pub fn inverse(&self) -> Option<Self> {
        Some(Blade(self.mv().inverse()?))
    }

    /// Returns whether the blade approximately equals zero.
    pub fn is_zero(&self) -> bool {
        self.mv().is_zero()
    }
    /// Returns whether the blade is zero or represents an object with zero
    /// radius.
    pub fn is_degenerate(&self) -> bool {
        approx_eq(&self.mag2(), &0.0)
    }
    /// Given an OPNS-form hypersphere/hyperplane, returns `true` if it is a
    /// hyperplane or flat point and `false` if it is a hypersphere or finite
    /// point pair.
    pub fn opns_is_flat(&self) -> bool {
        // TODO: It should be possible to optimize this significantly.

        // Wedge with Ni. Hyperplanes contain the point at infinity while
        // hyperspheres do not.
        (self ^ Blade::NI).is_zero()
    }
    /// Given an IPNS-form hypersphere/hyperplane, returns `true` if it is a
    /// hyperplane or flat point and `false` if it is a hypersphere or finite
    /// point pair.
    pub fn ipns_is_flat(&self) -> bool {
        // TODO: It should be possible to optimize this significantly.

        // Dot with Ni. Hyperplanes contain the point at infinity while
        // hyperspheres do not.
        (Blade::NI << self).is_zero()
    }

    /// Given an IPNS-form hypersphere/hyperplane, query whether a point is
    /// inside, outside, or on the hypersphere/hyperplane.
    pub fn ipns_query_point(&self, point: impl ToConformalPoint) -> PointWhichSide {
        let blade = point.to_normalized_1blade();
        let dot = self.dot(&blade);
        match approx_cmp(&dot, &0.0) {
            std::cmp::Ordering::Less => PointWhichSide::Outside,
            std::cmp::Ordering::Equal => PointWhichSide::On,
            std::cmp::Ordering::Greater => PointWhichSide::Inside,
        }
    }

    /// Returns the Ni component of a 1-blade.
    ///
    /// See https://w.wiki/6L8q
    pub fn ni(&self) -> Float {
        (self.mv()[Axes::E_MINUS] + self.mv()[Axes::E_PLUS]) / 2.0
    }
    /// Returns the No component of a 1-blade.
    ///
    /// See https://w.wiki/6L8q
    pub fn no(&self) -> Float {
        self.mv()[Axes::E_MINUS] - self.mv()[Axes::E_PLUS]
    }

    /// Returns the minimum number of Euclidean dimensions of the space
    /// containing the object represented by the blade.
    pub fn ndim(&self) -> u8 {
        self.mv().ndim()
    }
    /// Returns the grade of the blade, which is the number of basis vectors in
    /// each term, or 0 if the object is degenerate.
    pub fn grade(&self) -> u8 {
        match self.mv().terms().first() {
            Some(term) => term.grade(),
            None => 0,
        }
    }

    /// Returns `true` if the blade is a null vector (has zero magnitude).
    pub fn is_null_vector(&self) -> bool {
        approx_eq(&self.mag2(), &0.0)
    }
    /// Returns `true` if the object represented by an IPNS blade is imaginary
    /// (has negative magnitude).
    pub fn ipns_is_imaginary(&self) -> bool {
        is_approx_negative(&self.ipns_mag2())
    }
    /// Returns `true` if the object represented by an IPNS blade is real (has
    /// positive magnitude).
    pub fn ipns_is_real(&self) -> bool {
        is_approx_positive(&self.ipns_mag2())
    }
    /// Returns `true` if the object represented by an OPNS blade is imaginary.
    pub fn opns_is_imaginary(&self) -> bool {
        is_approx_negative(&self.opns_mag2())
    }
    /// Returns `true` if the object represented by an OPNS blade is real.
    pub fn opns_is_real(&self) -> bool {
        is_approx_positive(&self.opns_mag2())
    }

    /// Returns the squared radius of the hypersphere represented by an IPNS
    /// 1-blade, or `None` the object is flat.
    pub fn ipns_radius2(&self) -> Option<Float> {
        let no = self.no();
        util::try_div(self.mag2(), no * no)
    }
    /// Returns the radius of the hypersphere represented by an IPNS 1-blade, or
    /// `None` the object is flat or imaginary. This is negative for
    /// "inside-out" spheres.
    pub fn ipns_radius(&self) -> Option<Float> {
        if self.ipns_is_flat() {
            None
        } else {
            util::try_div(util::try_sqrt(self.mag2())?, self.no())
        }
    }
    /// Returns the point at the center of the hypersphere represented by an
    /// IPNS 1-blade, or `None` if the object is flat.
    #[track_caller]
    pub fn ipns_sphere_center(&self) -> Point {
        self.to_point()
    }

    /// Returns the distance from the origin to the closest point on the
    /// hyperplane represented by an IPNS 1-blade. This distance is negative if
    /// the plane's normal vector faces toward the origin. Returns `None` if the
    /// object is a hypersphere centered at the origin.
    pub fn ipns_plane_distance(&self) -> Option<Float> {
        let v = self.to_vector();
        util::try_div(-self.ni(), v.mag())
    }
    /// Returns the vector from the origin to the closest point on the
    /// hyperplane represented by an IPNS 1-blade.
    pub fn ipns_plane_pole(&self) -> Vector {
        let v = self.to_vector();
        util::try_div(&v * self.ni(), v.mag2()).unwrap_or(Vector::EMPTY)
    }
    /// Returns the normal vector of the hyperplane represented by an IPNS
    /// 1-blade.
    pub fn ipns_plane_normal(&self) -> Option<Vector> {
        (-self.to_vector()).normalize()
    }

    /// Reflects IPNS `obj` across the hyperplane/hypershpere represented by an IPNS
    /// 1-blade.
    #[must_use]
    pub fn ipns_reflect_ipns(&self, obj: &Blade) -> Blade {
        let sign = if obj.grade() % 2 == 0 { 1.0 } else { -1.0 };
        Blade(self.mv() * obj.mv() * self.mv() * sign)
    }
    /// Reflects OPNS `obj` across the hypersphere/hyperplane represented by an
    /// IPNS 1-blade.
    #[must_use]
    pub fn ipns_reflect_opns(&self, obj: &Blade) -> Blade {
        self.ipns_reflect_ipns(&-obj)
    }

    /// Returns the vector from the origin to the point represented by an OPNS
    /// 1-blade.
    #[track_caller]
    pub fn to_point(&self) -> Point {
        if self.is_zero() {
            return Point::Degenerate;
        }
        assert_eq!(1, self.grade(), "expected 1-blade; got {self}");
        match util::try_div(self.to_vector(), self.no()) {
            Some(p) => Point::Finite(p),
            None => Point::Infinity,
        }
    }
    /// Converts an OPNS flat point (point pair containing the point at
    /// infinity) to an OPNS point.
    ///
    /// # Panics
    ///
    /// This method panics if the blade is not a 2-blade (or zero).
    #[track_caller]
    pub fn flat_point_to_point(&self) -> Point {
        if self.is_zero() {
            return Point::Degenerate;
        }
        assert_eq!(2, self.grade(), "expected 1-blade; got {self}");
        (Blade::NO << self).to_point()
    }
    /// Factors an OPNS point pair into two OPNS points. Returns `None` if the
    /// point pair is degenerate or imaginary.
    ///
    /// # Panics
    ///
    /// This method panics if the blade is not a 2-blade (or zero).
    pub fn point_pair_to_points(&self) -> Option<[Point; 2]> {
        if self.is_zero() {
            return None;
        }
        assert_eq!(2, self.grade(), "expected 2-blade; got {self}");
        if self.opns_is_flat() {
            let finite_point = self.flat_point_to_point();
            if self.mv()[Axes::E_PLANE] < 0.0 {
                Some([Point::Infinity, finite_point])
            } else {
                Some([finite_point, Point::Infinity])
            }
        } else {
            let radius = Term::scalar(self.mag()?);
            let multiplier = (Blade::NI << self).inverse()?;
            Some([
                Blade((self.mv() - radius) * multiplier.mv()).to_point(),
                Blade((self.mv() + radius) * multiplier.mv()).to_point(),
            ])
        }
    }

    /// Returns the scale factor between `self` and `other` if they differ by a
    /// scalar factor, or `None` if they do not or if either is zero.
    pub fn scale_factor_to(&self, other: &Self) -> Option<Float> {
        if self.grade() != other.grade() || self.mv().is_zero() || other.mv().is_zero() {
            return None;
        }

        let factor = self.unchecked_scale_factor_to(other);

        for term in self.mv().terms() {
            let scaled_self_coef = term.coef * factor;
            let other_coef = other.mv()[term.axes];
            if !approx_eq(&scaled_self_coef, &other_coef) {
                return None;
            }
        }
        for term in other.mv().terms() {
            if !term.is_zero() && self.mv().get(term.axes).is_none() {
                // `other` has a nonzero term that `self` doesn't have.
                return None;
            }
        }

        Some(factor)
    }
    /// Returns the scale factor between `self` and `other`, assuming they
    /// differ by a scalar factor. If they do not, then the result is undefined.
    pub fn unchecked_scale_factor_to(&self, other: &Self) -> Float {
        // Pick a term to compare.
        let i = self.mv().most_significant_term().axes;
        // Compute the factor between those terms.
        other.mv()[i] / self.mv()[i]
    }

    /// Returns the blade as a vector in Euclidean space, ignoring non-Euclidean
    /// components.
    pub fn to_vector(&self) -> Vector {
        let axis_iter = 0..self.ndim();
        let components = axis_iter.map(|i| self.mv()[Axes::euclidean(i)]);
        components.collect()
    }

    /// Returns the scalar product of two blades.
    pub fn dot(&self, other: &Self) -> Float {
        self.mv().dot(other.mv())
    }
    /// Returns the squared magnitude of an OPNS blade that is negative iff the
    /// object is imaginary.
    fn opns_mag2(&self) -> Float {
        -self.ipns_mag2()
    }
    /// Returns the squared magnitude of an IPNS blade that is negative iff the
    /// object is imaginary.
    fn ipns_mag2(&self) -> Float {
        let sign = match self.grade() % 4 {
            0 | 1 => 1.0,
            2 | 3 => -1.0,
            _ => unreachable!(),
        };
        self.mag2() * sign
    }
    /// Returns the absolute value of the magnitude of the blade.
    pub fn abs_mag2(&self) -> Float {
        self.mag2().abs()
    }
    /// Returns raw the squared magnitude of the blade.
    fn mag2(&self) -> Float {
        self.dot(self)
    }
    /// Returns the magnitude of the blade.
    fn mag(&self) -> Option<Float> {
        util::try_sqrt(self.mag2())
    }
}

/// Point on the one-point compactification of N-dimensional Euclidean space.
#[derive(Debug, Clone, PartialEq)]
pub enum Point {
    /// Finite point.
    Finite(Vector),
    /// Point at infinity.
    Infinity,
    /// Degenerate point, represented by the zero blade.
    Degenerate,
}
impl fmt::Display for Point {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Point::Finite(p) => fmt::Display::fmt(p, f),
            Point::Infinity => write!(f, "∞ "),
            Point::Degenerate => write!(f, "<degenerate>"),
        }
    }
}
impl Default for Point {
    fn default() -> Self {
        Self::Finite(Vector::EMPTY)
    }
}
impl approx::AbsDiffEq for Point {
    type Epsilon = Float;

    fn default_epsilon() -> Self::Epsilon {
        Vector::default_epsilon()
    }

    fn abs_diff_eq(&self, other: &Self, epsilon: Self::Epsilon) -> bool {
        match (self, other) {
            (Point::Finite(a), Point::Finite(b)) => a.abs_diff_eq(b, epsilon),
            _ => self == other,
        }
    }
}
impl Point {
    /// Point at the origin.
    pub const ORIGIN: Self = Point::Finite(Vector::EMPTY);

    /// Returns the point if it is finite, and `None` otherwise.
    pub fn to_finite(self) -> Result<Vector> {
        match self {
            Point::Finite(p) => Ok(p),
            Point::Infinity => bail!("expected finite point; got infinite point"),
            Point::Degenerate => bail!("expected finite point; got degenerate point"),
        }
    }

    /// Returns the point if it is finite, or panics otherwise.
    #[track_caller]
    pub fn unwrap(self) -> Vector {
        self.to_finite().unwrap()
    }
}

/// Trait to convert to a point in the conformal geometric algebra.
pub trait ToConformalPoint {
    /// Returns the representation of a point in the conformal geometric
    /// algebra.
    fn to_normalized_1blade(self) -> Blade;
}
impl<V: VectorRef> ToConformalPoint for V {
    fn to_normalized_1blade(self) -> Blade {
        Blade::vector_to_point(self)
    }
}
impl ToConformalPoint for &'_ Blade {
    fn to_normalized_1blade(self) -> Blade {
        self.normalize_point()
    }
}
impl ToConformalPoint for Blade {
    fn to_normalized_1blade(self) -> Blade {
        self.normalize_point()
    }
}
impl ToConformalPoint for &'_ Point {
    fn to_normalized_1blade(self) -> Blade {
        match self {
            Point::Finite(p) => Blade::point(p),
            Point::Infinity => Blade::NI,
            Point::Degenerate => Blade::ZERO,
        }
    }
}
impl ToConformalPoint for Point {
    fn to_normalized_1blade(self) -> Blade {
        (&self).to_normalized_1blade()
    }
}
