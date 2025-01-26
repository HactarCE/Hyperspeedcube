use std::fmt;
use std::ops::{Mul, MulAssign};

use super::*;
use crate::*;

/// [Isometry](https://w.wiki/7SP4) in space represented by a multivector --
/// i.e., some composition of translations, rotations, and reflections.
///
/// In Euclidean space, this is either a direct isometry composed of rotations
/// and translations ("rigid" transformations) or an opposite isometry composed
/// of a direct isometric and a single reflection.
#[derive(Debug, Clone, PartialEq)]
pub struct Isometry(pub(crate) Multivector);

impl Default for Isometry {
    fn default() -> Self {
        Isometry::ident()
    }
}

impl fmt::Display for Isometry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.mv())
    }
}

impl approx::AbsDiffEq for Isometry {
    type Epsilon = Float;

    fn default_epsilon() -> Self::Epsilon {
        EPSILON
    }

    fn abs_diff_eq(&self, other: &Self, epsilon: Self::Epsilon) -> bool {
        self.0.abs_diff_eq(&other.0, epsilon) || self.0.abs_diff_eq(&-&other.0, epsilon)
    }
}

impl AsMultivector for Isometry {
    fn mv(&self) -> &Multivector {
        &self.0
    }
    fn into_mv(self) -> Multivector {
        self.0
    }
}

impl Isometry {
    /// Returns the identity isometry.
    pub fn ident() -> Self {
        Isometry(Multivector::scalar(1.0))
    }

    /// Returns whether this is the identity isometry.
    pub fn is_ident(&self) -> bool {
        self.canonicalize()
            .is_some_and(|i| approx_eq(&i, &Self::ident()))
    }

    /// Returns the minimum number of Euclidean dimensions that this isometry
    /// requires.
    pub fn ndim(&self) -> u8 {
        self.0.ndim()
    }
    /// Returns whether the isometry include an odd number of reflections.
    pub fn is_reflection(&self) -> bool {
        match self.0.nonzero_terms().next() {
            Some(term) => term.grade() % 2 == 1,
            None => false, // bad isometry (should be nonzero)
        }
    }

    /// Constructs a reflection through a vector. Returns `None` if the vector
    /// is zero.
    pub fn from_reflection(v: impl VectorRef) -> Option<Self> {
        Some(Self::from_reflection_normalized(v.normalize()?))
    }
    /// Constructs a reflection through a vector.
    ///
    /// `v` **must** be a unit vector.
    pub fn from_reflection_normalized(v: impl VectorRef) -> Self {
        Isometry(Multivector::from(v))
    }

    /// Constructs a rotation from one vector to another. Returns `None` if the
    /// vectors are directly opposite one another or either is zero.
    ///
    /// This method normalizes its inputs.
    pub fn from_vec_to_vec(a: impl VectorRef, b: impl VectorRef) -> Option<Self> {
        Self::from_vec_to_vec_normalized(&a.normalize()?, &b.normalize()?)
    }
    /// Constructs a rotation from one vector to another. Returns `None` if the
    /// vectors are directly opposite one another or either is zero.
    ///
    /// `a` and `b` **must** be unit vectors.
    pub fn from_vec_to_vec_normalized(a: &Vector, b: &Vector) -> Option<Self> {
        let avg = (a + b).normalize()?;
        Some(Self::from_vector_product_normalized(a, avg))
    }

    /// Constructs a rotation from an angle in an axis-aligned plane.
    ///
    /// If the axes are the same, returns the identity.
    pub fn from_angle_in_axis_plane(a: u8, b: u8, angle: Float) -> Self {
        Self::from_angle_in_normalized_plane(Vector::unit(a), Vector::unit(b), angle)
    }
    /// Constructs a rotation from an angle in a plane defined by two vectors.
    ///
    /// `a` and `b` **must** be perpendicular unit vectors.
    pub fn from_angle_in_normalized_plane(
        a: impl VectorRef,
        b: impl VectorRef,
        angle: Float,
    ) -> Self {
        let half_angle = angle / 2.0;
        let cos = half_angle.cos();
        let sin = half_angle.sin();
        Self::from_vector_product_normalized(&a, a.scale(cos) + b.scale(sin))
    }

    /// Constructs a rotation from a product of two vectors. Returns `None` if
    /// either vector is zero.
    ///
    /// This constructs a rotation of **double** the angle between them.
    ///
    /// `a` and `b` **must** be normalized.
    pub fn from_vector_product(a: impl VectorRef, b: impl VectorRef) -> Option<Self> {
        Some(Self::from_vector_product_normalized(
            a.normalize()?,
            b.normalize()?,
        ))
    }
    /// Constructs a rotation from a product of two vectors.
    ///
    /// This constructs a rotation of **double** the angle between them.
    pub fn from_vector_product_normalized(a: impl VectorRef, b: impl VectorRef) -> Self {
        let mut m = Multivector::from(b) * Multivector::from(a);
        m.retain_terms(|term| !term.is_zero());
        Isometry(m)
    }

    /// Returns the reverse isometry.
    #[must_use]
    pub fn reverse(&self) -> Isometry {
        Isometry(self.0.reverse())
    }

    /// Returns the rotation matrix for an isometry in Euclidean space which
    /// keeps the origin fixed.
    ///
    /// The result is undefined for any other isometry.
    pub fn euclidean_rotation_matrix(&self) -> Matrix {
        Matrix::from_cols(self.euclidean_matrix_cols(self.ndim()))
    }
    /// Returns the projective transformation matrix for an isometry in
    /// `ndim`-dimensional Euclidean space.
    ///
    /// The result is undefined for any other isometry.
    pub fn euclidean_projective_transformation_matrix(&self, ndim: u8) -> Matrix {
        let w = self.0.sandwich_blade(&Blade::NO).to_vector();
        let cols = self
            .euclidean_matrix_cols(ndim)
            .map(|col| col - &w)
            .chain(std::iter::once(Vector::unit(ndim) + &w))
            .with_exact_size(ndim as usize + 1);
        Matrix::from_cols(cols)
    }
    fn euclidean_matrix_cols(&self, ndim: u8) -> impl '_ + ExactSizeIterator<Item = Vector> {
        (0..ndim).map(|i| {
            self.0.sandwich_term_euclidean(Term {
                coef: 1.0,
                axes: Axes::euclidean(i),
            })
        })
    }

    /// Transforms another isometry by this one.
    #[must_use]
    pub fn transform_isometry(&self, other: &Isometry) -> Isometry {
        Isometry(self.transform(&other.0))
    }
    /// Transforms another isometry by this one, reversing it if this is a
    /// reflection.
    #[must_use]
    pub fn transform_isometry_uninverted(&self, other: &Isometry) -> Isometry {
        let ret = self.transform_isometry(other);
        if self.is_reflection() {
            ret.reverse()
        } else {
            ret
        }
    }
    /// Transforms a vector by the isometry.
    pub fn transform_vector(&self, v: impl VectorRef) -> Vector {
        self.transform_blade(&Blade::vector(v)).to_vector()
    }
    /// Transforms a point by the isometry.
    pub fn transform_point(&self, p: impl ToConformalPoint) -> Point {
        self.transform_blade(&p.to_1blade()).to_point()
    }
    /// Transforms an OPNS blade by the isometry.
    pub fn transform_blade(&self, b: &Blade) -> Blade {
        // TODO: we wouldn't need this funny special case if sandwich_blade()
        // used the conjugate instead of the reverse!
        let sandwich_product = self.0.sandwich_blade(b);
        if self.is_reflection() {
            -sandwich_product
        } else {
            sandwich_product
        }
    }
    /// Transforms a multivector by the isometry.
    pub fn transform(&self, m: &Multivector) -> Multivector {
        self.0.sandwich(m)
    }

    /// Returns the magnitude of the isometry as a real number.
    fn unsigned_mag(&self) -> Float {
        self.mag2().abs().sqrt()
    }
    /// Returns the squared magnitude of the isometry, which should always be
    /// `1` for a direct isometry and `-1` for an opposite isometry.
    fn mag2(&self) -> Float {
        self.dot(&self.reverse())
    }

    /// Normalizes the isometry so that the magnitude is `1`.
    pub fn normalize(&self) -> Option<Isometry> {
        let mag = self.unsigned_mag();
        if approx_eq(&mag, &0.0) {
            return None;
        }

        let multiplier = mag.recip();

        Some(Isometry(
            self.0.nonzero_terms().map(|term| term * multiplier).sum(),
        ))
    }
    /// Normalizes the isometry so that the magnitude is `1` and the first
    /// nonzero component is positive, or returns `None` if the isometry is
    /// zero (which is invalid).
    #[must_use]
    pub fn canonicalize(&self) -> Option<Isometry> {
        let mag = self.unsigned_mag();
        if approx_eq(&mag, &0.0) || !mag.is_finite() {
            return None;
        }

        let sign_of_first_term = self.0.nonzero_terms().next()?.coef.signum();
        let multiplier = mag.recip() * sign_of_first_term;

        Some(Isometry(
            self.0.nonzero_terms().map(|term| term * multiplier).sum(),
        ))
    }

    /// Returns the scalar product of two isometries.
    pub fn dot(&self, other: &Isometry) -> Float {
        self.0.dot(&other.0)
    }
    /// Interpolates between two (normalized) isometries and normalizes the
    /// output.
    pub fn nlerp(a: &Isometry, b: &Isometry, t: Float) -> Isometry {
        // Math stolen from https://docs.rs/cgmath/latest/src/cgmath/quaternion.rs.html
        let self_t = 1.0 - t;
        let other_t = t * a.dot(b).signum();
        Isometry(&a.0 * self_t + &b.0 * other_t)
            .canonicalize()
            .unwrap_or_else(|| if t < 0.5 { a.clone() } else { b.clone() })
    }
    /// Spherically interpolates between two (normalized) isometries.
    pub fn slerp(a: &Isometry, b: &Isometry, t: Float) -> Isometry {
        // Math stolen from https://docs.rs/cgmath/latest/src/cgmath/quaternion.rs.html

        let mut dot = a.dot(b);
        // Negate the second isometry sometimes.
        let sign = dot.signum();
        dot = dot.abs();

        const NLERP_THRESHOLD: Float = 0.9995;
        if dot > NLERP_THRESHOLD {
            // Optimization: Use nlerp for nearby isometries.
            return Self::nlerp(a, b, t);
        }

        // Stay within the domain of `acos()`.
        let robust_dot = dot.clamp(-1.0, 1.0);
        let angle = robust_dot.acos();
        let scale1 = (angle * (1.0 - t)).sin();
        let scale2 = (angle * t).sin() * sign; // Reverse the second isometry if negative dot product

        Isometry(&a.0 * scale1 + &b.0 * scale2)
            .canonicalize()
            .unwrap_or_else(|| if t < 0.5 { a.clone() } else { b.clone() })
    }
    /// Returns a rotation matrix that interpolates between two (normalized)
    /// isometries. This gives a better result than `slerp` or `nlerp` when
    /// there is a reflection between the two isometries.
    pub fn interpolate_euclidean_rotation(a: &Isometry, b: &Isometry, t: Float) -> Matrix {
        if a.is_reflection() == b.is_reflection() {
            Self::slerp(a, b, t).euclidean_rotation_matrix()
        } else {
            util::lerp(
                a.euclidean_rotation_matrix(),
                b.euclidean_rotation_matrix(),
                t,
            )
        }
    }
    /// Returns a projective transformation matrix that interpolates between two
    /// (normalized) isometries. This gives a better result than `slerp` or
    /// `nlerp` when there is a reflection between the two isometries.
    pub fn interpolate_euclidean_projective_transformation(
        a: &Isometry,
        b: &Isometry,
        t: Float,
        ndim: u8,
    ) -> Matrix {
        if a.is_reflection() == b.is_reflection() {
            Self::slerp(a, b, t).euclidean_projective_transformation_matrix(ndim)
        } else {
            util::lerp(
                a.euclidean_projective_transformation_matrix(ndim),
                b.euclidean_projective_transformation_matrix(ndim),
                t,
            )
        }
    }
}

impl From<Isometry> for Multivector {
    fn from(versor: Isometry) -> Self {
        versor.0
    }
}

/// Compose versors.
impl Mul for &Isometry {
    type Output = Isometry;

    fn mul(self, rhs: Self) -> Self::Output {
        Isometry(&self.0 * &rhs.0)
    }
}
impl_forward_bin_ops_to_ref! {
    impl Mul for Isometry { fn mul() }
}
impl_forward_assign_ops_to_owned! {
    impl MulAssign for Isometry { fn mul_assign() { * } }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_isometry_sandwich_signs() {
        let s = Blade::scalar(1.0);
        let v1 = Blade::vector(vector![1.0, 2.0, 3.0]);
        let v2 = Blade::vector(vector![-3.0, 2.0, 1.0]);
        let v3 = Blade::vector(vector![1.0, -2.0, 3.0]);
        let v4 = Blade::vector(vector![0.0, 0.0, 0.0, 1.0]);
        let bv = &v1 ^ &v2;
        let tv = &v1 ^ &v2 ^ &v3;
        let qv = &v1 ^ &v2 ^ &v3 ^ &v4;
        assert!(!bv.is_null_vector());
        assert!(!tv.is_null_vector());
        assert!(!qv.is_null_vector());

        let ident = Isometry::ident();
        assert_approx_eq!(ident.transform_blade(&s), s);
        assert_approx_eq!(ident.transform_blade(&v1), v1);
        assert_approx_eq!(ident.transform_blade(&bv), bv);
        assert_approx_eq!(ident.transform_blade(&tv), tv);
        assert_approx_eq!(ident.transform_blade(&qv), qv);

        let refl_v = Isometry::from_reflection_normalized(Vector::unit(4));
        assert_approx_eq!(refl_v.transform_blade(&s), -&s);
        assert_approx_eq!(refl_v.transform_blade(&v1), v1);
        assert_approx_eq!(refl_v.transform_blade(&v2), v2);
        assert_approx_eq!(refl_v.transform_blade(&v3), v3);
        assert_approx_eq!(refl_v.transform_blade(&v4), v4);
        assert_approx_eq!(refl_v.transform_blade(&bv), -&bv);
        assert_approx_eq!(refl_v.transform_blade(&tv), tv);
        assert_approx_eq!(refl_v.transform_blade(&qv), -&qv);

        let refl_z = Isometry::from_reflection_normalized(vector![0.0, 0.0, 1.0]);
        assert_approx_eq!(refl_z.transform_blade(&s), -&s);
        assert_approx_eq!(
            refl_z.transform_blade(&v1),
            Blade::vector(vector![1.0, 2.0, -3.0]),
        );
        let v1_new = refl_z.transform_blade(&v1);
        let v2_new = refl_z.transform_blade(&v2);
        let v3_new = refl_z.transform_blade(&v3);
        let v4_new = refl_z.transform_blade(&v4);
        assert_approx_eq!(refl_z.transform_blade(&bv), -&v1_new ^ &v2_new);
        assert_approx_eq!(refl_z.transform_blade(&tv), &v1_new ^ &v2_new ^ &v3_new);
        assert_approx_eq!(
            refl_z.transform_blade(&qv),
            -&v1_new ^ &v2_new ^ &v3_new ^ &v4_new,
        );

        let rot_xy =
            Isometry::from_vec_to_vec_normalized(&vector![1.0], &vector![0.0, 1.0]).unwrap();
        assert_approx_eq!(rot_xy.transform_blade(&s), s);
        assert_approx_eq!(
            rot_xy.transform_blade(&v1),
            Blade::vector(vector![-2.0, 1.0, 3.0]),
        );
        let v1_new = rot_xy.transform_blade(&v1);
        let v2_new = rot_xy.transform_blade(&v2);
        let v3_new = rot_xy.transform_blade(&v3);
        let v4_new = rot_xy.transform_blade(&v4);
        assert_approx_eq!(rot_xy.transform_blade(&bv), &v1_new ^ &v2_new);
        assert_approx_eq!(rot_xy.transform_blade(&tv), &v1_new ^ &v2_new ^ &v3_new);
        assert_approx_eq!(
            rot_xy.transform_blade(&qv),
            &v1_new ^ &v2_new ^ &v3_new ^ &v4_new,
        );

        let rot_xy_refl_z = &rot_xy * &refl_z;
        assert_approx_eq!(rot_xy_refl_z.transform_blade(&s), -&s);
        assert_approx_eq!(
            rot_xy_refl_z.transform_blade(&v1),
            Blade::vector(vector![-2.0, 1.0, -3.0]),
        );
        let v1_new = rot_xy_refl_z.transform_blade(&v1);
        let v2_new = rot_xy_refl_z.transform_blade(&v2);
        let v3_new = rot_xy_refl_z.transform_blade(&v3);
        let v4_new = rot_xy_refl_z.transform_blade(&v4);
        assert_approx_eq!(rot_xy_refl_z.transform_blade(&bv), -&v1_new ^ &v2_new);
        assert_approx_eq!(
            rot_xy_refl_z.transform_blade(&tv),
            &v1_new ^ &v2_new ^ &v3_new,
        );
        assert_approx_eq!(
            rot_xy_refl_z.transform_blade(&qv),
            -&v1_new ^ &v2_new ^ &v3_new ^ &v4_new,
        );
    }
}
