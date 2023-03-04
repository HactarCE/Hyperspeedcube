//! Versors describing conformal transformations in Euclidean space.

use std::ops::{Mul, MulAssign};

use super::Multivector;
use crate::math::{util, Matrix, Vector, VectorRef};

/// Rotor describing a rotation in Euclidean space.
#[derive(Debug, Clone, PartialEq)]
pub struct Rotor(Multivector);

impl Default for Rotor {
    fn default() -> Self {
        Rotor::ident()
    }
}

impl approx::AbsDiffEq for Rotor {
    type Epsilon = f32;

    fn default_epsilon() -> Self::Epsilon {
        crate::math::EPSILON
    }

    fn abs_diff_eq(&self, other: &Self, epsilon: Self::Epsilon) -> bool {
        self.0.abs_diff_eq(&other.0, epsilon) || self.0.abs_diff_eq(&-&other.0, epsilon)
    }
}

impl Rotor {
    /// Returns the identity rotor.
    pub fn ident() -> Self {
        Self(Multivector::scalar(1.0))
    }
    /// Constructs a rotor that transforms one vector to another, or returns
    /// `None` if the vectors are directly opposite one another.
    ///
    /// This method normalizes its inputs.
    pub fn from_vec_to_vec(a: impl VectorRef, b: impl VectorRef) -> Option<Self> {
        let a = a.normalize()?;
        let b = b.normalize()?;
        let avg = (b + &a).normalize()?;
        Some(Self::from_vector_product(a, avg))
    }
    /// Constructs a rotor from a product of two vectors.
    ///
    /// This constructs a rotation of **double** the angle between them.
    pub fn from_vector_product(a: impl VectorRef, b: impl VectorRef) -> Self {
        Self(Multivector::from(b) * Multivector::from(a))
    }
    /// Constructs a rotor from an angle in an axis-aligned plane.
    ///
    /// If the axes are the same, returns the identity.
    pub fn from_angle_in_axis_plane(a: u8, b: u8, angle: f32) -> Self {
        Self::from_angle_in_plane(Vector::unit(a), Vector::unit(b), angle)
    }
    /// Constructs a rotor from an angle in a plane defined by two vectors.
    ///
    /// The vectors are assumed to be perpendicular and normalized.
    pub fn from_angle_in_plane(a: impl VectorRef, b: impl VectorRef, angle: f32) -> Self {
        let half_angle = angle / 2.0;
        let cos = half_angle.cos();
        let sin = half_angle.sin();
        Self::from_vector_product(&a, a.scale(cos) + b.scale(sin))
    }
    /// Constructs a rotor directly from a multivector. The multivector is
    /// assumed to be normalized.
    ///
    /// # Panics
    ///
    /// This function panics in debug mode if the multivector contains any
    /// element with an odd number of axes. **Only use it if you are sure that
    /// the multivector is from the even subalgebra.**
    pub fn from_multivector(m: Multivector) -> Self {
        debug_assert!(m.terms().iter().all(|term| term.axes.count_ones() % 2 == 0));
        Self(m)
    }

    /// Returns the rotor's internal multivector.
    pub fn multivector(&self) -> &Multivector {
        &self.0
    }
    /// Returns the scalar component of the rotor.
    pub fn s(&self) -> f32 {
        self.0[0]
    }
    /// Returns the angle of the rotation represented by the rotor in radians.
    pub fn angle(&self) -> f32 {
        self.s().acos() * 2.0
    }
    /// Returns the angle of the rotation represented by the rotor in radians,
    /// in the range 0 to PI.
    pub fn abs_angle(&self) -> f32 {
        self.s().abs().acos() * 2.0
    }

    /// Returns the reverse rotor.
    #[must_use]
    pub fn reverse(&self) -> Rotor {
        Self(self.0.reverse())
    }

    /// Returns the matrix for the rotor.
    pub fn matrix(&self) -> Matrix {
        self.0.matrix()
    }

    /// Transforms another rotor using this one.
    #[must_use]
    pub fn transform_rotor(&self, other: &Rotor) -> Rotor {
        Rotor(self.0.sandwich_multivector(&other.0))
    }
    /// Transforms a vector using the rotor.
    pub fn transform_vector(&self, vector: impl VectorRef) -> Vector {
        self.0.sandwich_vector(vector)
    }

    /// Returns the magnitude of the rotor, which should always be one for
    /// rotors representing pure rotations.
    pub fn mag(&self) -> f32 {
        self.dot(&self.reverse()).sqrt()
    }
    /// Normalizes the rotor so that the magnitude is one and the scalar
    /// component is positive, or returns `None` if the rotor is zero.
    #[must_use]
    pub fn normalize(mut self) -> Option<Rotor> {
        let mult = self.s().signum() / self.mag();
        if !mult.is_finite() {
            return None;
        }
        self.0 *= mult;
        Some(self)
    }

    /// Returns the scalar product of two rotors.
    pub fn dot(&self, other: &Rotor) -> f32 {
        self.0.dot(&other.0)
    }
    /// Returns the angle between two rotors, in the range 0 to PI.
    pub fn angle_to(&self, other: &Rotor) -> f32 {
        (self.reverse() * other).abs_angle()
    }
    /// Interpolates between two (normalized) rotors and normalizes the output.
    pub fn nlerp(&self, other: &Rotor, t: f32) -> Rotor {
        // Math stolen from https://docs.rs/cgmath/latest/src/cgmath/quaternion.rs.html
        let self_t = 1.0 - t;
        let other_t = if self.dot(other).is_sign_positive() {
            t
        } else {
            -t
        };
        Self(&self.0 * self_t + &other.0 * other_t)
            .normalize()
            .unwrap_or_else(|| other.clone())
    }
    /// Spherically interpolates between two (normalized) rotors.
    pub fn slerp(&self, other: &Rotor, t: f32) -> Rotor {
        // Math stolen from https://docs.rs/cgmath/latest/src/cgmath/quaternion.rs.html

        let mut dot = self.dot(other);
        // Negate the second rotor sometimes.
        let sign = dot.signum();
        dot = dot.abs();

        const NLERP_THRESHOLD: f32 = 0.9995;
        if dot > NLERP_THRESHOLD {
            // Optimization: Use nlerp for nearby rotors.
            return self.nlerp(other, t);
        }

        // Stay within the domain of `acos()`.
        let robust_dot = dot.clamp(-1.0, 1.0);
        let angle = robust_dot.acos();
        let scale1 = (angle * (1.0 - t)).sin();
        let scale2 = (angle * t).sin() * sign; // Reverse the second rotor if negative dot product

        Self(&self.0 * scale1 + &other.0 * scale2)
            .normalize()
            .unwrap_or_else(|| other.clone()) // should never happen
    }
}

impl From<Rotor> for Multivector {
    fn from(rotor: Rotor) -> Self {
        rotor.0
    }
}

/// Compose rotors.
impl<'a> Mul for &'a Rotor {
    type Output = Rotor;

    fn mul(self, rhs: Self) -> Self::Output {
        Rotor(&self.0 * &rhs.0)
    }
}
impl_forward_bin_ops_to_ref! {
    impl Mul for Rotor { fn mul() }
}
impl_forward_assign_ops_to_owned! {
    impl MulAssign for Rotor { fn mul_assign() { * } }
}

/// Rotoreflector desrcibing a rotation and optional reflection in Euclidean space.
#[derive(Debug, Clone, PartialEq)]
pub struct Rotoreflector {
    matrix: Matrix,
    multivector: Multivector,
}
impl Default for Rotoreflector {
    fn default() -> Self {
        Self::ident()
    }
}
impl approx::AbsDiffEq for Rotoreflector {
    type Epsilon = f32;

    fn default_epsilon() -> Self::Epsilon {
        crate::math::EPSILON
    }

    fn abs_diff_eq(&self, other: &Self, epsilon: Self::Epsilon) -> bool {
        self.multivector.abs_diff_eq(&other.multivector, epsilon)
            || self.multivector.abs_diff_eq(&-&other.multivector, epsilon)
    }
}
impl From<Rotor> for Rotoreflector {
    fn from(rotor: Rotor) -> Self {
        Multivector::from(rotor).into()
    }
}
impl From<Multivector> for Rotoreflector {
    fn from(m: Multivector) -> Self {
        Self {
            matrix: m.matrix(),
            multivector: m,
        }
    }
}
impl Rotoreflector {
    /// Returns the identity rotoreflector.
    pub fn ident() -> Self {
        Self {
            matrix: Matrix::EMPTY_IDENT,
            multivector: Multivector::scalar(1.0),
        }
    }

    /// Returns whether this rotoreflector results in an odd number of
    /// reflections.
    pub fn is_reflection(&self) -> bool {
        match self.multivector.terms().get(0) {
            Some(term) => term.axes.count_ones() % 2 == 1,
            None => {
                // wtf? this shouldn't happen.
                false
            }
        }
    }

    /// Constructs a rotoreflector from a single reflection.
    pub fn from_reflection(v: impl VectorRef) -> Self {
        Self {
            matrix: Matrix::from_reflection(&v),
            multivector: v.into(),
        }
    }

    /// Returns the matrix for the transformation.
    pub fn matrix(&self) -> &Matrix {
        &self.matrix
    }

    /// Returns the reverse transformation.
    pub fn reverse(&self) -> Self {
        self.multivector.reverse().into()
    }

    /// Transforms another rotor using this one.
    #[must_use]
    pub fn transform_rotor(&self, other: &Rotor) -> Rotor {
        Rotor(self.multivector.sandwich_multivector(&other.0))
    }

    /// Transforms another rotoreflector using this one.
    #[must_use]
    pub fn transform_rotoreflector(&self, other: &Rotoreflector) -> Rotoreflector {
        (self.multivector.sandwich_multivector(&other.multivector)).into()
    }

    /// Transforms another rotoreflector using this one, reversing it if this is
    /// a reflection.
    pub fn transform_rotoreflector_uninverted(&self, other: &Rotoreflector) -> Rotoreflector {
        let ret = self.transform_rotoreflector(other);
        if self.is_reflection() {
            ret.reverse()
        } else {
            ret
        }
    }

    /// Interpolates between the identity and this rotoreflector.
    pub fn interpolate(&self, t: f32) -> Matrix {
        if self.is_reflection() {
            util::mix(&Matrix::ident(self.matrix.ndim()), &self.matrix, t)
        } else {
            Rotor::ident()
                .slerp(&Rotor(self.multivector.clone()), t)
                .matrix()
        }
    }
}
