//! General unoptimized geometric algebra implementation.
//!
//! This is only used for generating animations, so it doesn't need to be fast.
//! _I hope._

use std::fmt;
use std::iter::Sum;
use std::ops::{Add, AddAssign, Mul};

use crate::math::*;

const AXIS_NAMES: &[char] = &['X', 'Y', 'Z', 'W', 'U', 'V', 'R', 'S'];

/// Term in the geometric algebra, consisting of a real coefficient and a
/// bitmask representing the unit blade.
#[derive(Debug, Default, Copy, Clone, PartialEq)]
pub struct Blade {
    /// Coefficient.
    coef: f32,
    /// Bitset of axes.
    axes: u32,
}
impl fmt::Display for Blade {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.coef.fmt(f)?;
        write!(f, "{}", axes_to_string(self.axes))?;
        Ok(())
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

        let ret = Blade {
            coef: self.coef * rhs.coef * sign,
            axes: self.axes ^ rhs.axes, // Common axes cancel.
        };
        ret
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
}

/// Sum of blades in the geometric algebra. Blades are stored sorted by their
/// `axes` bitmask. No two blades in one multivector may have the same set of
/// axes.
#[derive(Default, Clone, PartialEq)]
pub struct Multivector(Vec<Blade>);
impl<V: VectorRef<f32>> From<V> for Multivector {
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
impl fmt::Debug for Multivector {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut ret = f.debug_struct("Multivector");
        for term in &self.0 {
            let field_name = if term.axes == 0 {
                "S".to_string() // scalar
            } else {
                axes_to_string(term.axes)
            };
            ret.field(&field_name, &term.coef);
        }
        ret.finish()
    }
}
impl AddAssign<Blade> for Multivector {
    fn add_assign(&mut self, rhs: Blade) {
        match self.0.binary_search_by_key(&rhs.axes, |term| term.axes) {
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
        for &term in &rhs.0 {
            ret += term;
        }
        ret
    }
}
impl<'a> Mul<Blade> for &'a Multivector {
    type Output = Multivector;

    fn mul(self, rhs: Blade) -> Self::Output {
        let mut ret = Multivector::zero();
        for &term in &self.0 {
            ret += term * rhs;
        }
        ret
    }
}
impl<'a> Mul<Blade> for Multivector {
    type Output = Multivector;

    fn mul(self, rhs: Blade) -> Self::Output {
        &self * rhs
    }
}
impl<'a> AddAssign<&'a Multivector> for Multivector {
    fn add_assign(&mut self, rhs: &'a Multivector) {
        for &term in &rhs.0 {
            *self += term;
        }
    }
}
impl<'a> Add<&'a Multivector> for Multivector {
    type Output = Multivector;

    fn add(mut self, rhs: &'a Multivector) -> Self::Output {
        self += rhs;
        self
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
impl<'a> Mul<f32> for &'a Multivector {
    type Output = Multivector;

    fn mul(self, rhs: f32) -> Self::Output {
        let mut ret = self.clone();
        for term in &mut ret.0 {
            term.coef *= rhs;
        }
        ret
    }
}
impl Sum for Multivector {
    fn sum<I: Iterator<Item = Self>>(mut iter: I) -> Self {
        let mut ret = iter.next().unwrap_or_default();
        for elem in iter {
            ret += &elem;
        }
        ret
    }
}
impl Multivector {
    /// Zero multivector.
    pub const ZERO: Self = Self(vec![]);

    /// Returns the zero multivector.
    pub fn zero() -> Self {
        Self::ZERO
    }
    /// Returns the lexicographically largest axis mask in the multivector.
    fn largest_axis_mask(&self) -> u32 {
        match self.0.last() {
            Some(term) => term.axes,
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
                .binary_search_by_key(&(1 << ndim), |term| term.axes)
                .unwrap_or_else(|i| i),
        );
    }
    /// Returns a component of the multivector, or possibly `None` if it is
    /// zero.
    pub fn get(&self, axes: u32) -> Option<f32> {
        self.0
            .binary_search_by_key(&axes, |term| term.axes)
            .ok()
            .map(|i| self.0[i].coef)
    }
}

/// Rotor describing a rotation in an arbitrary number of dimensions.
#[derive(Debug, Clone, PartialEq)]
pub struct Rotor(Multivector);
impl Default for Rotor {
    fn default() -> Self {
        Rotor::identity()
    }
}
impl Rotor {
    /// Returns the identity rotor.
    pub fn identity() -> Self {
        Self(Multivector(vec![Blade::scalar(1.0)]))
    }
    /// Constructs a rotor from a product of two vectors.
    ///
    /// This constructs a rotation of DOUBLE the angle between them.
    pub fn from_vector_product(a: impl VectorRef<f32>, b: impl VectorRef<f32>) -> Self {
        Self(&Multivector::from(a) * &Multivector::from(b))
    }
    /// Constructs a rotor from an angle in an axis-aligned plane.
    ///
    /// If the axes are the same, returns the identity.
    pub fn from_angle_in_axis_plane(a: u8, b: u8, angle: f32) -> Self {
        Self::from_angle_in_plane(&Vector::unit(a), &Vector::unit(b), angle)
    }
    /// Constructs a rotor from an angle in a plane defined by two vectors.
    ///
    /// The vectors are assumed to be perpendicular and normalized.
    pub fn from_angle_in_plane(a: &Vector<f32>, b: &Vector<f32>, angle: f32) -> Self {
        let half_angle = angle / 2.0;
        let cos = half_angle.cos();
        let sin = half_angle.sin();
        Self::from_vector_product(a, a * cos + b * sin)
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
        let mut ret = self.clone();
        for term in &mut ret.0 .0 {
            match term.axes.count_ones() % 4 {
                0 | 3 => (),
                1 | 2 => term.coef = -term.coef,
                _ => unreachable!(),
            }
        }
        ret
    }

    /// Returns the matrix for the rotor.
    pub fn matrix(&self) -> Matrix<f32> {
        Matrix::from_cols((0..self.ndim()).map(|axis| self.transform_vector(Vector::unit(axis))))
    }

    /// Transforms another rotor using this one.
    #[must_use]
    pub fn transform_rotor(&self, other: &Rotor) -> Rotor {
        // This can be unwrapped for efficiency.
        Self(&(&self.0 * &other.0) * &self.reverse().0)
    }

    /// Transforms a vector using the rotor.
    pub fn transform_vector(&self, vector: impl VectorRef<f32>) -> Vector<f32> {
        let rv = &self.reverse().0 * &Multivector::from(vector);
        let ret = &rv * &self.0;
        (0..ret.ndim())
            .map(|i| ret.get(1 << i).unwrap_or(0.0))
            .collect()
    }

    /// Returns the magnitude of the rotor, which should always be one for
    /// rotors representing pure rotations.
    pub fn mag(&self) -> f32 {
        (self.reverse() * self).s().sqrt()
    }
    /// Normalizes the rotor so that the magnitude is one.
    #[must_use]
    pub fn normalize(mut self) -> Option<Rotor> {
        let mult = self.s().signum() / self.mag();
        if !mult.is_finite() {
            return None;
        }
        for term in &mut self.0 .0 {
            term.coef *= mult;
        }
        Some(self)
    }

    /// Multiplies the rotor by a scalar.
    pub fn scale(&self, factor: f32) -> Self {
        Self(&self.0 * Blade::scalar(factor))
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
        Self(&(&self.0 * self_t) + &(&other.0 * other_t))
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

        Self(&self.scale(scale1).0 + &other.scale(scale2).0)
            .normalize()
            .unwrap_or_else(|| other.clone()) // should never happen
    }
}
impl<'a> Mul for &'a Rotor {
    type Output = Rotor;

    fn mul(self, rhs: Self) -> Self::Output {
        Rotor(&self.0 * &rhs.0)
    }
}
impl_forward_bin_ops_to_ref!(impl Mul for Rotor { fn mul() });

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
    fn gen_vector(ndim: u8) -> impl Strategy<Value = Vector<f32>> {
        proptest::collection::vec(gen_reasonable_float(), ndim as usize)
            .prop_map(|xs| Vector(xs.into_iter().map(|x| x as f32).collect()))
    }
    fn gen_normalized_vector(ndim: u8) -> impl Strategy<Value = Vector<f32>> {
        gen_vector(ndim).prop_filter_map("cannot normalize zero vector", |v| v.normalise())
    }
    fn gen_simple_rotor(ndim: u8) -> impl Strategy<Value = Rotor> {
        [gen_normalized_vector(ndim), gen_normalized_vector(ndim)]
            .prop_map(|[a, b]| Rotor::from_vector_product(a, b))
    }

    fn assert_rotor_transform_vector(
        rotor: &Rotor,
        input_vector: &Vector<f32>,
        expected: &Vector<f32>,
    ) -> Result<(), TestCaseError> {
        let result = rotor.transform_vector(input_vector);
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
            let halfway = (&a + &b).normalise();
            prop_assume!(halfway.is_some());
            let halfway = halfway.unwrap();
            let rotor = Rotor::from_vector_product(&a, &halfway);

            let v_mag = vec.mag();
            assert_rotor_transform_vector(&rotor, &(&a * v_mag), &(&b * v_mag))?;
            assert_rotor_transform_vector(&rotor, &-&(&a * v_mag), &-&(&b * v_mag))?;

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
            let v = (&b - &u * u.dot(&b)).normalise().unwrap();
            let u_mag = vec.dot(&u);
            let v_mag = vec.dot(&v);
            let expected = &vec
                + u * (u_mag * (cos - 1.0) - v_mag * sin)
                + v * (u_mag * sin + v_mag * (cos - 1.0));
            assert_rotor_transform_vector(&rotor, &vec, &expected)?;
        }


        #[test]
        fn proptest_rotor_times_rotor(
            rotors in proptest::collection::vec(gen_simple_rotor(7), 1..=4),
            vec in gen_vector(7)
        ) {
            let mut combined = rotors[0].clone();
            let mut expected = rotors[0].transform_vector(&vec);
            for r in &rotors[1..] {
                combined = &combined * r;
                expected = r.transform_vector(expected);
            }

            assert_rotor_transform_vector(&combined, &vec, &expected)?;
        }
    }
}
