use std::fmt;

use crate::*;

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
    pub fn to_finite(self) -> Result<Vector, Point> {
        match self {
            Point::Finite(v) => Ok(v),
            p => Err(p),
        }
    }

    /// Returns the point if it is finite, or panics otherwise.
    #[track_caller]
    pub fn unwrap(self) -> Vector {
        self.to_finite().expect("expected point")
    }
    /// Returns the point if it is finite, or panics otherwise.
    #[track_caller]
    pub fn expect(self, msg: &str) -> Vector {
        self.to_finite().expect(msg)
    }
}

/// Trait to convert to a point in the conformal geometric algebra.
pub trait ToConformalPoint {
    /// Returns the OPNS representation of a point in the conformal geometric
    /// algebra.
    fn to_normalized_1blade(self) -> Blade;
}
impl<V: VectorRef> ToConformalPoint for V {
    /// Constructs the OPNS blade representing a point.
    ///
    /// See https://w.wiki/6L8o
    fn to_normalized_1blade(self) -> Blade {
        // p + NO + 1/2 * NI * ||p||^2
        let mag2 = self.mag2();
        Blade(Multivector::from(self) + Multivector::NO + Multivector::NI * 0.5 * mag2)
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
