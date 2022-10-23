//! General unoptimized geometric algebra implementation.
//!
//! This is only used for generating animations, so it doesn't need to be fast.
//! _I hope._

use std::fmt;
use std::iter::Sum;
use std::ops::{Add, AddAssign, Mul, MulAssign, Neg};

use crate::math::*;

const AXIS_NAMES: &[char] = &['X', 'Y', 'Z', 'W', 'U', 'V', 'R', 'S'];

/// Term in the geometric algebra, consisting of a real coefficient and a
/// bitmask representing the unit blade.
#[derive(Debug, Default, Copy, Clone, PartialEq)]
pub struct Blade {
    /// Coefficient.
    pub coef: f32,
    /// Bitset of axes.
    pub axes: u32,
}
impl fmt::Display for Blade {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.coef.fmt(f)?;
        write!(f, "{}", axes_to_string(self.axes))?;
        Ok(())
    }
}
impl approx::AbsDiffEq for Blade {
    type Epsilon = f32;

    fn default_epsilon() -> Self::Epsilon {
        super::EPSILON
    }

    fn abs_diff_eq(&self, other: &Self, epsilon: Self::Epsilon) -> bool {
        self.axes == other.axes && self.coef.abs_diff_eq(&other.coef, epsilon)
    }
}
impl Mul for Blade {
    type Output = Blade;

    fn mul(self, rhs: Self) -> Self::Output {
        // Count the number of swaps needed to sort the combined product. If the
        // number of swaps is odd, negate the result.
        let mut neg = false;
        let mut a = self.axes;
        let mut b = rhs.axes;
        while a != 0 && b != 0 {
            let i = b.trailing_zeros() + 1;
            a >>= i;
            b >>= i;
            neg ^= a.count_ones() & 1 != 0;
        }
        let sign = if neg { -1.0 } else { 1.0 };

        Blade {
            coef: self.coef * rhs.coef * sign,
            axes: self.axes ^ rhs.axes, // Common axes cancel.
        }
    }
}
impl Blade {
    /// Constructs a scalar blade.
    pub fn scalar(x: f32) -> Self {
        Self { coef: x, axes: 0 }
    }

    /// Returns the grade of the blade, which is the number of dimensions in its
    /// subspace. Every grade can be represented as an exterior product of no
    /// fewer than _r_ vectors, where _r_ is the blade's grade.
    pub fn grade(self) -> u8 {
        self.axes.count_ones() as u8
    }

    /// Returns an iterator over the even basis blades up to some grade.
    pub fn even_bases(max_grade: u8) -> impl Iterator<Item = Self> {
        (0..1 << max_grade)
            .map(|axes| Self { coef: 1.0, axes })
            .filter(|blade| blade.grade() % 2 == 0)
    }

    /// Returns the conjugate blade, which has the order of its axes
    /// conceptually reversed. In practice, we simply negate the coefficient if
    /// necessary.
    pub fn conjugate(mut self) -> Self {
        match self.axes.count_ones() % 4 {
            0 | 3 => (),
            1 | 2 => self.coef = -self.coef,
            _ => unreachable!(),
        }
        self
    }
}

/// Sum of blades in the geometric algebra. Blades are stored sorted by their
/// `axes` bitmask. No two blades in one multivector may have the same set of
/// axes.
#[derive(Default, Clone, PartialEq)]
pub struct Multivector(Vec<Blade>);
impl<V: VectorRef> From<V> for Multivector {
    fn from(vec: V) -> Self {
        Self(
            vec.iter()
                .enumerate()
                .map(|(i, v)| Blade {
                    coef: v,
                    axes: 1 << i,
                })
                .collect(),
        )
    }
}
impl approx::AbsDiffEq for Multivector {
    type Epsilon = f32;

    fn default_epsilon() -> Self::Epsilon {
        super::EPSILON
    }

    fn abs_diff_eq(&self, other: &Self, epsilon: Self::Epsilon) -> bool {
        let largest_axis_mask = std::cmp::max(self.largest_axis_mask(), other.largest_axis_mask());
        (0..=largest_axis_mask).all(|i| {
            let a = self.get(i).unwrap_or(0.0);
            let b = other.get(i).unwrap_or(0.0);
            a.abs_diff_eq(&b, epsilon)
        })
    }
}
impl From<Blade> for Multivector {
    fn from(blade: Blade) -> Self {
        Self(vec![blade])
    }
}
impl fmt::Debug for Multivector {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut ret = f.debug_struct("Multivector");
        for blade in &self.0 {
            if blade.coef != 0.0 {
                let field_name = if blade.axes == 0 {
                    "S".to_string() // scalar
                } else {
                    axes_to_string(blade.axes)
                };
                ret.field(&field_name, &blade.coef);
            }
        }
        ret.finish()
    }
}
impl AddAssign<Blade> for Multivector {
    fn add_assign(&mut self, rhs: Blade) {
        match self.0.binary_search_by_key(&rhs.axes, |blade| blade.axes) {
            Ok(i) => self.0[i].coef += rhs.coef,
            Err(i) => self.0.insert(i, rhs),
        }
    }
}
impl Add<Blade> for Multivector {
    type Output = Multivector;

    fn add(mut self, rhs: Blade) -> Self::Output {
        self += rhs;
        self
    }
}
impl<'a> Add<Blade> for &'a Multivector {
    type Output = Multivector;

    fn add(self, rhs: Blade) -> Self::Output {
        self.clone() + rhs
    }
}
impl<'a> Add for &'a Multivector {
    type Output = Multivector;

    fn add(self, rhs: Self) -> Self::Output {
        let mut ret = self.clone();
        for &blade in &rhs.0 {
            ret += blade;
        }
        ret
    }
}
impl<'a> Mul<Blade> for &'a Multivector {
    type Output = Multivector;

    fn mul(self, rhs: Blade) -> Self::Output {
        self.0.iter().map(|&blade| blade * rhs).sum()
    }
}
impl Mul<Blade> for Multivector {
    type Output = Multivector;

    fn mul(self, rhs: Blade) -> Self::Output {
        &self * rhs
    }
}
impl<'a> AddAssign<&'a Multivector> for Multivector {
    fn add_assign(&mut self, rhs: &'a Multivector) {
        for &blade in &rhs.0 {
            *self += blade;
        }
    }
}
impl AddAssign for Multivector {
    fn add_assign(&mut self, rhs: Self) {
        *self += &rhs;
    }
}
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
impl Mul<f32> for Multivector {
    type Output = Multivector;

    fn mul(mut self, rhs: f32) -> Self::Output {
        for blade in &mut self.0 {
            blade.coef *= rhs;
        }
        self
    }
}
impl<'a> Mul<f32> for &'a Multivector {
    type Output = Multivector;

    fn mul(self, rhs: f32) -> Self::Output {
        self.clone() * rhs
    }
}
impl Neg for Multivector {
    type Output = Multivector;

    fn neg(self) -> Self::Output {
        self * -1.0
    }
}
impl<'a> Neg for &'a Multivector {
    type Output = Multivector;

    fn neg(self) -> Self::Output {
        self * -1.0
    }
}
impl Sum<Blade> for Multivector {
    fn sum<I: Iterator<Item = Blade>>(iter: I) -> Self {
        iter.fold(Multivector::ZERO, |a, b| a + b)
    }
}
impl Sum for Multivector {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.fold(Multivector::ZERO, |a, b| a + b)
    }
}
impl Multivector {
    /// Zero multivector.
    pub const ZERO: Self = Self(vec![]);

    /// Returns a scalar multivector.
    pub fn scalar(s: f32) -> Self {
        Self(vec![Blade::scalar(s)])
    }
    /// Returns the zero multivector.
    pub fn zero() -> Self {
        Self::ZERO
    }
    /// Returns the lexicographically largest axis mask in the multivector.
    fn largest_axis_mask(&self) -> u32 {
        match self.0.last() {
            Some(blade) => blade.axes,
            None => 0,
        }
    }
    /// Returns the maximum grade (number of dimensions) of the multivector.
    pub fn ndim(&self) -> u8 {
        32 - self.largest_axis_mask().leading_zeros() as u8
    }
    /// Truncates the multivector to a maximum grade (number of dimensions).
    pub fn truncate_to_ndim(&mut self, ndim: u8) {
        self.0.truncate(
            self.0
                .binary_search_by_key(&(1 << ndim), |blade| blade.axes)
                .unwrap_or_else(|i| i),
        );
    }
    /// Returns a component of the multivector, or possibly `None` if it is
    /// zero.
    pub fn get(&self, axes: u32) -> Option<f32> {
        self.0
            .binary_search_by_key(&axes, |blade| blade.axes)
            .ok()
            .map(|i| self.0[i].coef)
    }

    /// Returns the blade components of the multivector.
    pub fn blades(&self) -> &[Blade] {
        &self.0
    }

    /// Returns the conjugate multivector, which has the order of its axes
    /// conceptually reversed. In practice, we simply negate some of the terms.
    pub fn conjugate(&self) -> Self {
        Self(self.0.iter().copied().map(Blade::conjugate).collect())
    }

    /// Returns the sandwich product with a vector: `M * v * M_rev`.
    pub fn sandwich_vector(&self, v: impl VectorRef) -> Vector {
        let ndim = std::cmp::max(self.ndim(), v.ndim());
        (0..ndim)
            .map(|i| self.sandwich_axis_vector(i, v.get(i)))
            .sum()
    }
    /// Returns the sandwich product with a multivector: `M * R * M_rev`.
    pub fn sandwich_multivector(&self, multivector: &Multivector) -> Multivector {
        self.0
            .iter()
            .flat_map(|&lhs| {
                multivector.0.iter().flat_map(move |&mid| {
                    self.0.iter().map(move |&rhs| lhs * mid * rhs.conjugate())
                })
            })
            .sum()
    }
    /// Returns the matrix equivalent to a sandwich product with the
    /// multivector.
    ///
    /// The matix is more expensive to compute initially than any one
    pub fn matrix(&self) -> Matrix {
        Matrix::from_cols((0..self.ndim()).map(|axis| self.sandwich_axis_vector(axis, 1.0)))
    }
    /// Returns the sandwich product with an axis-aligned vector: `M * v *
    /// M_rev`.
    fn sandwich_axis_vector(&self, axis: u8, mag: f32) -> Vector {
        let ndim = std::cmp::max(self.ndim(), axis + 1);
        let mid = Blade {
            coef: mag,
            axes: 1 << axis,
        };

        let mut ret = Vector::zero(ndim);
        for &lhs in &self.0 {
            for &rhs in &self.0 {
                let blade = lhs * mid * rhs.conjugate();
                if blade.axes.count_ones() == 1 {
                    ret[blade.axes.trailing_zeros() as u8] += blade.coef;
                }
            }
        }
        ret
    }
}
impl_forward_bin_ops_to_ref! {
    impl Add for Multivector { fn add() }
    impl Mul for Multivector { fn mul() }
}
impl_forward_assign_ops_to_owned! {
    impl MulAssign for Multivector { fn mul_assign() { * } }
}

/// Rotor describing a rotation in an arbitrary number of dimensions.
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
        super::EPSILON
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
    /// Constructs a rotor that transforms one vector to another.
    pub fn from_vec_to_vec(a: impl VectorRef, b: impl VectorRef) -> Option<Self> {
        let a = a.normalize()?;
        let b = b.normalize()?;
        let avg = (b + &a).normalize()?;
        Some(Self::from_vector_product(a, avg))
    }
    /// Constructs a rotor from a product of two vectors.
    ///
    /// This constructs a rotation of DOUBLE the angle between them.
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
        debug_assert!(m.0.iter().all(|blade| blade.axes.count_ones() % 2 == 0));
        Self(m)
    }

    /// Returns the rotor's internal multivector.
    pub fn multivector(&self) -> &Multivector {
        &self.0
    }
    /// Returns the number of dimensions of the rotor.
    pub fn ndim(&self) -> u8 {
        self.0.ndim()
    }
    /// Returns the scalar (dot product) component of the rotor.
    pub fn s(&self) -> f32 {
        self.0.get(0).unwrap_or(0.0)
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
        Self(self.0.conjugate())
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
        (self.reverse() * self).s().sqrt()
    }
    /// Normalizes the rotor so that the magnitude is one and the scalar
    /// component is positive.
    #[must_use]
    pub fn normalize(mut self) -> Option<Rotor> {
        let mult = self.s().signum() / self.mag();
        if !mult.is_finite() {
            return None;
        }
        for blade in &mut self.0 .0 {
            blade.coef *= mult;
        }
        Some(self)
    }

    /// Returns the dot product between two rotors.
    pub fn dot(&self, other: &Rotor) -> f32 {
        let largest_axis_mask =
            std::cmp::min(self.0.largest_axis_mask(), other.0.largest_axis_mask());
        (0..=largest_axis_mask)
            .filter_map(|axes| Some(self.0.get(axes)? * other.0.get(axes)?))
            .sum()
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
    fn from(r: Rotor) -> Self {
        r.0
    }
}
impl<'a> Mul for &'a Rotor {
    type Output = Rotor;

    fn mul(self, rhs: Self) -> Self::Output {
        Rotor(&self.0 * &rhs.0)
    }
}
impl_forward_bin_ops_to_ref! {
    impl Mul for Rotor { fn mul() }
}

/// Transformation consisting of a rotation and an optional reflection.
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
        super::EPSILON
    }

    fn abs_diff_eq(&self, other: &Self, epsilon: Self::Epsilon) -> bool {
        self.multivector.abs_diff_eq(&other.multivector, epsilon)
            || self.multivector.abs_diff_eq(&-&other.multivector, epsilon)
    }
}
impl From<Rotor> for Rotoreflector {
    fn from(r: Rotor) -> Self {
        Multivector::from(r).into()
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
        match self.multivector.blades().get(0) {
            Some(blade) => blade.axes.count_ones() % 2 == 1,
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
        self.multivector.conjugate().into()
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
}

impl<V: VectorRef> Mul<V> for &Rotoreflector {
    type Output = Vector;

    fn mul(self, rhs: V) -> Self::Output {
        &self.matrix * rhs
    }
}
impl<V: VectorRef> Mul<V> for Rotoreflector {
    type Output = Vector;

    fn mul(self, rhs: V) -> Self::Output {
        self.matrix * rhs
    }
}
impl Mul for &Rotoreflector {
    type Output = Rotoreflector;

    fn mul(self, rhs: Self) -> Self::Output {
        (&self.multivector * &rhs.multivector).into()
    }
}
impl_forward_bin_ops_to_ref! {
    impl Mul for Rotoreflector { fn mul() }
}

fn axes_to_string(axes: u32) -> String {
    std::iter::successors(Some(axes), |a| Some(a >> 1))
        .take_while(|&a| a != 0)
        .enumerate()
        .filter(|&(_, a)| a & 1 != 0)
        .map(|(i, _)| AXIS_NAMES.get(i).copied().unwrap_or('?'))
        .collect()
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;

    const EPSILON: f32 = 0.001;

    fn gen_reasonable_float() -> impl Strategy<Value = f32> {
        (any::<u8>(), any::<i8>()).prop_map(|(f, i)| i as f32 + f as f32 / 256.0)
    }
    fn gen_vector(ndim: u8) -> impl Strategy<Value = Vector> {
        proptest::collection::vec(gen_reasonable_float(), ndim as usize)
            .prop_map(|xs| Vector(xs.into_iter().rev().map(|x| x as f32).collect()))
    }
    fn gen_normalized_vector(ndim: u8) -> impl Strategy<Value = Vector> {
        gen_vector(ndim).prop_filter_map("cannot normalize zero vector", |v| v.normalize())
    }
    fn gen_simple_rotor(ndim: u8) -> impl Strategy<Value = Rotor> {
        [gen_normalized_vector(ndim), gen_normalized_vector(ndim)]
            .prop_map(|[a, b]| Rotor::from_vector_product(a, b))
    }
    fn gen_rotor(ndim: u8) -> impl Strategy<Value = Rotor> {
        proptest::collection::vec(gen_simple_rotor(ndim), 1..=4)
            .prop_map(|rotors| rotors.into_iter().fold(Rotor::ident(), |a, b| a * b))
    }

    fn assert_rotor_transform_vector(
        rotor: &Rotor,
        input_vector: impl VectorRef,
        expected: impl VectorRef,
    ) -> Result<(), TestCaseError> {
        let result = rotor.transform_vector(&input_vector);
        prop_assert!(
            result.approx_eq(&expected, EPSILON) && result.ndim() == expected.ndim(),
            "\n\nrotor {rotor:?}\n\
             transforms {input_vector:?}\n\
             to {result:?}\n\
             but expected {expected:?}\n",
        );
        Ok(())
    }

    proptest! {
        #[test]
        fn proptest_rotor_transform_vector(
            a in gen_normalized_vector(7),
            b in gen_normalized_vector(7),
            vec in gen_vector(7),
        ) {
            let rotor = Rotor::from_vec_to_vec(&a, &b);
            prop_assume!(rotor.is_some());
            let rotor = rotor.unwrap();

            let v_mag = vec.mag();
            assert_rotor_transform_vector(&rotor, &a * v_mag, &b * v_mag)?;
            assert_rotor_transform_vector(&rotor, -(&a * v_mag), -(&b * v_mag))?;

            let cos = a.dot(&b);
            if cos.fract().abs() < EPSILON {
                return Ok(()); // skip
            }
            let angle = cos.acos();
            let sin = angle.sin();

            // Rotate the vector manually to verify that the rotor is doing what
            // we expect for vectors that are not entirely in its rotation
            // plane.
            let u = a;
            let v = (&b - &u * u.dot(&b)).normalize().unwrap();
            let u_mag = vec.dot(&u);
            let v_mag = vec.dot(&v);
            let expected = &vec
                + u * (u_mag * (cos - 1.0) - v_mag * sin)
                + v * (u_mag * sin + v_mag * (cos - 1.0));
            assert_rotor_transform_vector(&rotor, vec, expected)?;
        }

        #[test]
        fn proptest_rotor_times_rotor(
            rotors in proptest::collection::vec(gen_simple_rotor(7), 1..=4),
            vec in gen_vector(7)
        ) {
            let mut combined = rotors[0].clone();
            let mut expected = rotors[0].transform_vector(&vec);
            for r in &rotors[1..] {
                combined = r * combined;
                expected = r.transform_vector(expected);
            }

            assert_rotor_transform_vector(&combined, vec, expected)?;
        }

        #[test]
        fn proptest_rotor_transform_rotor(
            r1 in gen_rotor(4),
            r2 in gen_rotor(4),
            vec in gen_vector(4),
        ) {
            let expected = (&r1 * &r2).transform_vector(&vec);
            let transformed_rotor = r1.transform_rotor(&r2);
            let transformed_vector = r1.transform_vector(&vec);
            assert_rotor_transform_vector(&transformed_rotor, &transformed_vector, expected)?;
        }

        #[test]
        fn proptest_rotor_matrix(
            r in gen_rotor(7),
            vec in gen_vector(7),
        ) {
            let m = r.matrix();
            let expected = m * &vec;
            assert_rotor_transform_vector(&r, &vec, expected)?;
        }
    }
}
