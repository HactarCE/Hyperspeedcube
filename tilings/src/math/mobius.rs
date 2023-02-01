use cgmath::prelude::*;
use num_complex::Complex64 as Complex;
use std::ops::{Mul, MulAssign};

/// Mobius transformation
///
/// http://en.wikipedia.org/wiki/Mobius_transformation
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Mobius {
    pub a: Complex,
    pub b: Complex,
    pub c: Complex,
    pub d: Complex,
}
impl Mobius {
    /// Constructs a transformation that maps `z1` to zero, `z2` to one, and
    /// `z3` to infinity.
    ///
    /// http://en.wikipedia.org/wiki/Mobius_transformation#Mapping_first_to_0.2C_1.2C_.E2.88.9E
    ///
    /// If one of the `zi` is infinity, then the proper formula is obtained by
    /// first dividing all entries by `zi` and then taking the limit as `zi`
    /// approaches infinity.
    pub fn map_points(z1: Complex, z2: Complex, z3: Complex) -> Self {
        if z1.is_infinite() {
            Self {
                a: Complex::zero(),
                b: -(z2 - z3),
                c: -Complex::one(),
                d: z3,
            }
        } else if z2.is_infinite() {
            Self {
                a: Complex::one(),
                b: -z1,
                c: Complex::one(),
                d: -z3,
            }
        } else if z3.is_infinite() {
            Self {
                a: -Complex::one(),
                b: z1,
                c: Complex::zero(),
                d: -(z2 - z1),
            }
        } else {
            Self {
                a: z2 - z3,
                b: -z1 * (z2 - z3),
                c: z2 - z1,
                d: -z3 * (z2 - z1),
            }
        }
        .normalize()
    }

    /// Normalizes the Mobius transformation so that ad - bc = 1.
    #[must_use]
    pub fn normalize(self) -> Self {
        let k = Complex::one() / (self.a * self.d - self.b * self.c).sqrt();
        self * k
    }

    /// Returns the trace a + d of the Mobius transformation.
    pub fn trace(self) -> Complex {
        self.a + self.d
    }
    /// Returns the squared trace of the Mobius transformation.
    pub fn trace_sq(self) -> Complex {
        self.trace() * self.trace()
    }
}

impl<N: Copy> Mul<N> for Mobius
where
    Complex: MulAssign<N>,
{
    type Output = Self;

    fn mul(mut self, rhs: N) -> Self::Output {
        self.a *= rhs;
        self.b *= rhs;
        self.c *= rhs;
        self.d *= rhs;
        self
    }
}

impl Mul for Mobius {
    type Output = Mobius;

    fn mul(self, rhs: Self) -> Self::Output {
        let u = self;
        let v = rhs;

        Self {
            a: u.a * v.a + u.b * v.c,
            b: u.a * v.b + u.b * v.d,
            c: u.c * v.a + u.d * v.c,
            d: u.c * v.b + u.d * v.d,
        }
        .normalize()
    }
}

impl One for Mobius {
    fn one() -> Self {
        Self {
            a: Complex::one(),
            b: Complex::zero(),
            c: Complex::zero(),
            d: Complex::one(),
        }
    }
}
