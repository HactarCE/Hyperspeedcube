//! Multivectors of a single grade, which are called blades.

use std::fmt;
use std::ops::{BitXor, Mul, MulAssign, Neg, Shl};

use float_ord::FloatOrd;
use itertools::Itertools;

use crate::*;

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
/// The multivector inside `Blade` is assumed to never have terms with
/// coefficients that are approximately equal to zero. This is checked whenever
/// constructing a `Blade` from a `Multivector`.
///
/// # Panics
///
/// Many of the methods on this type will panic if passed a blade of the wrong
/// grade (zero is okay).
#[derive(Debug, Clone, PartialEq)]
pub struct Blade(Multivector);

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
        Blade::grade_project_from(Multivector::from(term), term.grade())
    }
}

impl From<Blade> for Multivector {
    fn from(value: Blade) -> Self {
        value.0
    }
}
impl TryFrom<Multivector> for Blade {
    type Error = MismatchedGrade;

    fn try_from(mut multivector: Multivector) -> std::result::Result<Self, Self::Error> {
        multivector.retain_terms(|term| !term.is_zero());
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
/// Error due to a multivector not having a consistent grade.
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, Hash)]
pub struct MismatchedGrade;
impl fmt::Display for MismatchedGrade {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "multivector has mismatched grade")
    }
}
impl std::error::Error for MismatchedGrade {}

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
        let grade = self.grade();
        Blade::grade_project_from(&self.0 * rhs, grade)
    }
}
impl Mul<Float> for Blade {
    type Output = Blade;

    fn mul(self, rhs: Float) -> Self::Output {
        let grade = self.grade();
        Blade::grade_project_from(self.0 * rhs, grade)
    }
}
impl MulAssign<Float> for Blade {
    fn mul_assign(&mut self, rhs: Float) {
        self.0 *= rhs;
    }
}

/// Outer product of a blade and a term.
///
/// See [Geometric algebra - Extensions of the inner and exterior
/// products](https://w.wiki/6L8p).
impl<'a> BitXor<Term> for &'a Blade {
    type Output = Blade;

    fn bitxor(self, rhs: Term) -> Self::Output {
        let grade = self.grade() + rhs.grade();
        Blade::grade_project_from(&self.0 ^ rhs, grade)
    }
}
impl BitXor<Term> for Blade {
    type Output = Blade;

    fn bitxor(self, rhs: Term) -> Self::Output {
        let grade = self.grade() + rhs.grade();
        Blade::grade_project_from(self.0 ^ rhs, grade)
    }
}
/// Outer product of a term and a blade.
///
/// See [Geometric algebra - Extensions of the inner and exterior
/// products](https://w.wiki/6L8p).
impl<'a> BitXor<&'a Blade> for Term {
    type Output = Blade;

    fn bitxor(self, rhs: &'a Blade) -> Self::Output {
        let grade = self.grade() + rhs.grade();
        Blade::grade_project_from(self ^ &rhs.0, grade)
    }
}
impl BitXor<Blade> for Term {
    type Output = Blade;

    fn bitxor(self, rhs: Blade) -> Self::Output {
        let grade = self.grade() + rhs.grade();
        Blade::grade_project_from(self ^ rhs.0, grade)
    }
}

/// Left contraction of a term and a blade.
///
/// See <https://youtu.be/oVyBbJl6xvo?t=180s> for an intuitive explanation.
impl<'a> Shl<&'a Blade> for Term {
    type Output = Blade;

    fn shl(self, rhs: &'a Blade) -> Self::Output {
        let grade = rhs.grade().saturating_sub(self.grade());
        Blade::grade_project_from(self << &rhs.0, grade)
    }
}
impl Shl<Blade> for Term {
    type Output = Blade;

    fn shl(self, rhs: Blade) -> Self::Output {
        let grade = rhs.grade().saturating_sub(self.grade());
        Blade::grade_project_from(self << rhs.0, grade)
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
        let grade = self.grade() + rhs.grade();
        Blade::grade_project_from(&self.0 ^ &rhs.0, grade)
    }
}
impl_forward_bin_ops_to_ref! {
    impl BitXor for Blade { fn bitxor() }
}

/// Left contraction of two blades.
///
/// See <https://youtu.be/oVyBbJl6xvo?t=180s> for an intuitive explanation.
impl<'a> Shl for &'a Blade {
    type Output = Blade;

    fn shl(self, rhs: Self) -> Self::Output {
        let grade = rhs.grade().saturating_sub(self.grade());
        Blade::grade_project_from(&self.0 << &rhs.0, grade)
    }
}
impl_forward_bin_ops_to_ref! {
    impl Shl for Blade { fn shl() }
}

impl_mul_sign!(impl Mul<Sign> for Blade);
impl_mulassign_sign!(impl MulAssign<Sign> for Blade);
impl Mul<Sign> for &Blade {
    type Output = Blade;

    fn mul(self, rhs: Sign) -> Self::Output {
        match rhs {
            Sign::Pos => self.clone(),
            Sign::Neg => -self,
        }
    }
}

impl Blade {
    /// Zero blade.
    ///
    /// This represents a degenerate object, which is not the same as the point
    /// at the origin.
    pub const ZERO: Self = Blade(Multivector::ZERO);

    /// Null vector representing the point at the origin.
    ///
    /// See [Conformal geometric algebra - Base and representation
    /// spaces](https://w.wiki/6L8q).
    pub const NO: Self = Blade(Multivector::NO);
    /// Null vector representing the point at infinity.
    ///
    /// See [Conformal geometric algebra - Base and representation
    /// spaces](https://w.wiki/6L8q).
    pub const NI: Self = Blade(Multivector::NI);

    /// Returns the Minkowski plane, defined as E=o∧∞.
    pub fn minkowski_plane() -> Self {
        Self(Multivector::minkowski_plane())
    }

    /// Grade-projects a multivector, keeping only nonzero terms with a specific
    /// grade.
    pub fn grade_project_from(mut m: Multivector, grade: u8) -> Self {
        m.retain_terms(|term| term.axes.count() == grade && !term.is_zero());
        Blade(m)
    }
    /// Constructs a scalar blade.
    pub fn scalar(s: Float) -> Self {
        Blade::grade_project_from(Multivector::scalar(s), 0)
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
        p.to_1blade()
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
        Blade::grade_project_from(mv * radius.signum(), 1)
    }
    /// Constructs an IPNS blade representing an imaginary hypersphere.
    pub fn ipns_imaginary_sphere(center: impl VectorRef, radius: Float) -> Self {
        let mv = Blade::point(center).mv() + Multivector::NI * radius * radius * 0.5;
        Blade::grade_project_from(mv * radius.signum(), 1)
    }
    /// Constructs an IPNS blade representing a hyperplane. If `distance` is
    /// positive, then the origin is considered "inside."
    pub fn ipns_plane(normal: impl VectorRef, distance: Float) -> Self {
        match normal.normalize() {
            // Negate so that the origin is "inside."
            Some(normal) => {
                let mv = -Multivector::from(normal) - Multivector::NI * distance;
                Blade::grade_project_from(mv, 1)
            }
            None => Blade::ZERO,
        }
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

    /// Returns the meet of two OPNS blades, give the number of dimensions of
    /// the whole space. This is the dual of the join or wedge operator.
    #[must_use]
    pub fn meet(a: &Blade, b: &Blade, ndim: u8) -> Self {
        let a_ipns = a.opns_to_ipns(ndim);
        let b_ipns = b.opns_to_ipns(ndim);
        (a_ipns ^ b_ipns).ipns_to_opns(ndim)
    }
    /// Returns the meet of two OPNS blades, give an OPNS blade representing the
    /// space they inhabit. This is the dual of the join or wedge operator.
    #[must_use]
    pub fn meet_in_space(a: &Blade, b: &Blade, opns_space: &Blade) -> Self {
        let a_ipns = a.opns_to_ipns_in_space(opns_space);
        let b_ipns = b.opns_to_ipns_in_space(opns_space);
        (a_ipns ^ b_ipns).ipns_to_opns_in_space(opns_space)
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
        let mut point_blade = point.to_1blade();
        point_blade *= point_blade.mv()[Axes::E_MINUS].signum(); // Normalize sign.
        let dot = self.dot(&point_blade);
        match approx_cmp(&dot, &0.0) {
            std::cmp::Ordering::Less => PointWhichSide::Outside,
            std::cmp::Ordering::Equal => PointWhichSide::On,
            std::cmp::Ordering::Greater => PointWhichSide::Inside,
        }
    }
    /// Given an OPNS-form manifold, query whether a point is on the manifold.
    pub fn opns_contains_point(&self, point: impl ToConformalPoint) -> bool {
        (self ^ point.to_1blade()).is_zero()
    }

    /// Returns the Ni component of a 1-blade.
    ///
    /// See [Conformal geometric algebra - Base and representation
    /// spaces](https://w.wiki/6L8q).
    pub fn ni(&self) -> Float {
        (self.mv()[Axes::E_MINUS] + self.mv()[Axes::E_PLUS]) / 2.0
    }
    /// Returns the No component of a 1-blade.
    ///
    /// See [Conformal geometric algebra - Base and representation
    /// spaces](https://w.wiki/6L8q).
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
        match self.mv().nonzero_terms().next() {
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
        Blade::grade_project_from(self.mv() * obj.mv() * self.mv() * sign, obj.grade())
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
                // These aren't actually valid blades, but we want to reuse
                // `Blade::to_point()`.
                Blade((self.mv() - radius) * multiplier.mv()).to_point(),
                Blade((self.mv() + radius) * multiplier.mv()).to_point(),
            ])
        }
    }

    /// Returns an object representing the tangent space of the manifold
    /// represented by an OPNS blade.
    pub fn opns_tangent_space(&self) -> TangentSpace {
        TangentSpace::from(self)
    }
    /// Projects a point onto the manifold, or returns `None` if the result is
    /// undefined.
    pub fn project_point(&self, p: &Point) -> Option<Point> {
        match p {
            Point::Finite(p) => {
                let pair = (Blade::point(p) ^ Blade::NI) << self << self;
                // The CGA projection operation actually gives us two points.
                let [a, b] = match pair.point_pair_to_points() {
                    Some(points) => points.map(|p| p.to_finite().ok()),
                    None => [None, None],
                };
                // Return whichever point is closer to `p`.
                crate::util::merge_options(a, b, |a, b| {
                    std::cmp::min_by_key(a, b, |q| FloatOrd((p - q).mag2()))
                })
                .map(Point::Finite)
            }
            Point::Infinity if self.opns_is_flat() => Some(Point::Infinity),
            Point::Infinity | Point::Degenerate => None,
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
