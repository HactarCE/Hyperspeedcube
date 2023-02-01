use cgmath::{prelude::*, Basis2, Deg};
use num_complex::Complex64 as Complex;
use std::hash::Hash;

use super::Polygon;
use crate::math::{self, Mobius};

/// Schlafli symbol for a regular tiling in 2D space, along with various
/// computed properties.
#[derive(Debug, Copy, Clone)]
pub struct Schlafli {
    /// Number of vertices in each tile.
    pub p: u8,
    /// Number of tiles around a vertex.
    pub q: u8,

    /// Induced geometry.
    pub geometry: Geometry,

    /// Side length opposite angle pi/p.
    pub p_side: f64,
    /// Side length opposite angle pi/q.
    pub q_side: f64,
    pub hypotenuse: f64,

    pub normalized_circumradius: f64,
}
impl PartialEq for Schlafli {
    fn eq(&self, other: &Self) -> bool {
        self.p == other.p && self.q == other.q
    }
}
impl Eq for Schlafli {}
impl Hash for Schlafli {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.p.hash(state);
        self.q.hash(state);
    }
}
impl Schlafli {
    /// Constructs the Schlafli symbol {p,q}.
    pub fn new(p: u8, q: u8) -> Self {
        let geometry = Geometry::from_schlafli(p, q);

        let right_angle = Deg(90.0);
        let p_angle = Deg(180.0) / p as f64;
        let q_angle = Deg(180.0) / q as f64;

        let p_side = geometry.triangle_side(p_angle, q_angle, right_angle);
        let q_side = geometry.triangle_side(q_angle, p_angle, right_angle);
        let hypotenuse = geometry.triangle_side(right_angle, p_angle, q_angle);

        let normalized_circumradius =
            geometry.projected_radius(hypotenuse) * geometry.disk_radius().unwrap_or(1.0);

        Self {
            p,
            q,

            geometry,

            p_side,
            q_side,
            hypotenuse,

            normalized_circumradius,
        }
    }

    /// Returns the transformation that centers the tiling on a vertex. This can
    /// be used to create a dual {q,p} tiling.
    pub fn vertex_mobius(self) -> Mobius {
        let mut angle = Deg(180.0) / self.q as f64;
        if self.q % 2 == 0 {
            angle *= 2.0;
        }
        let offset = Complex::cis(angle.0) * -self.normalized_circumradius;
        self.geometry.isometry(angle, offset.into())
    }

    /// Retruns the transformation that centers the tiling on an edge.
    pub fn edge_mobius(self) -> Mobius {
        let polygon = Polygon::new_regular(self);
        let offset = polygon.segments[0].midpoint().to_vec();

        let angle = Deg(180.0) / self.p as f64;
        let offset = Basis2::from_angle(-angle).rotate_vector(offset);

        // Convert to complex number.
        let offset = Complex::new(offset.x, offset.y);

        self.geometry.isometry(-angle, -offset)
    }
}

/// Type of 2D geometry.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum Geometry {
    Spherical,
    Euclidean,
    Hyperbolic,
}
impl Geometry {
    /// Radius of the disk used for the projection from spherical or hyperbolic
    /// space.
    const DISK_RADIUS: f64 = 1.0;
    /// Hypotenuse of the primitive triangle used to generate Euclidean tilings.
    const EUCLIDEAN_HYPOTENUSE: f64 = 1.0 / 3.0; // ZZZ - ??????????
                                                 // - Roice Nelson

    pub fn from_schlafli(p: u8, q: u8) -> Self {
        use std::cmp::Ordering::*;

        // {3,6} = Euclidean triangular tiling
        // {4,4} = Euclidean square tiling
        // {6,3} = Euclidean hexagonal tiling

        // 1/3+1/6 = 1/4+1/4 = 0.5

        let test = (p as f64).recip() + (q as f64).recip();

        match math::approx_cmp(&test, &0.5) {
            Less => Geometry::Hyperbolic,
            Equal => Geometry::Euclidean,
            Greater => Geometry::Spherical,
        }
    }

    /// Returns the side length opposite `alpha`. In the Euclidean case, `beta`
    /// and `gamma` are ignored.
    pub fn triangle_side(
        self,
        alpha: impl Angle<Unitless = f64>,
        beta: impl Angle<Unitless = f64>,
        gamma: impl Angle<Unitless = f64>,
    ) -> f64 {
        match self {
            // Spherical law of cosines
            Geometry::Spherical => {
                ((alpha.cos() + beta.cos() * gamma.cos()) / (beta.sin() * gamma.sin())).acos()
            }

            // Euclidean law of sines
            Geometry::Euclidean => alpha.sin() * Self::EUCLIDEAN_HYPOTENUSE,

            // Hyperbolic law of consines
            // http://en.wikipedia.org/wiki/Hyperbolic_law_of_cosines
            Geometry::Hyperbolic => {
                ((alpha.cos() + beta.cos() * gamma.cos()) / (beta.sin() * gamma.sin())).acosh()
            }
        }
    }

    /// Converts a distance from the origin in this geometry to Euclidean
    /// distance on the projection.
    pub fn projected_radius(self, norm: f64) -> f64 {
        match self {
            Geometry::Spherical => (norm * 0.5).tan(),
            Geometry::Euclidean => norm,
            Geometry::Hyperbolic => (norm * 0.5).tanh(),
        }
    }
    /// Converts a Euclidean distance from the origin in the projection to
    /// distance in this geometry.
    pub fn unprojected_radius(self, norm: f64) -> f64 {
        match self {
            Geometry::Spherical => norm.atan() * 2.0,
            Geometry::Euclidean => norm,
            Geometry::Hyperbolic => norm.atanh() * 2.0,
        }
    }
    /// Returns the radius of the disk used for projection, or `None` for
    /// Euclidean.
    pub fn disk_radius(self) -> Option<f64> {
        match self {
            Geometry::Euclidean => None,
            Geometry::Spherical | Geometry::Hyperbolic => Some(Self::DISK_RADIUS),
        }
    }

    /// Returns the Mobius transform that represents an isometry in this
    /// geometry. The isometry will rotate CCW by angle A about the origin, then
    /// translate the origin to P (and -P to the origin).
    pub fn isometry(self, angle: impl Angle<Unitless = f64>, p: Complex) -> Mobius {
        // As Don notes in the hyperbolic case:
        //
        // Any isometry of the Poincare disk can be expressed as a complex
        // function of z of the form: (T*z + P)/(1 + conj(P)*T*z), where T and P
        // are complex numbers, |P| < 1 and |T| = 1. This indicates a rotation
        // by T around the origin followed by moving the origin to P (and -P to
        // the origin).
        //
        // Roice figured out that the other cases can be handled with simple
        // variations of the C coefficients.
        let t = math::complex_cis(angle);

        let a = t;
        let b = p;
        let c = match self {
            Geometry::Spherical => -p.conj() * t,
            Geometry::Euclidean => Complex::zero(),
            Geometry::Hyperbolic => p.conj() * t,
        };
        let d = Complex::one();

        Mobius { a, b, c, d }
    }
}
