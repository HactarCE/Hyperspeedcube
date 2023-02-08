use serde::{Deserialize, Serialize};

use super::MathExpr;

/// Specification for a cut into a puzzle.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(untagged, deny_unknown_fields)]
pub enum CutSpec {
    /// Cut determined by a mathematical expression which returns a shape.
    Expr(MathExpr),
    /// (Hyper)spherical cut.
    Sphere {
        /// Center of the sphere.
        center: MathExpr,
        /// Radius of sphere, or multiple radii.
        radius: MathExpr,
    },
    /// (Hyper)planar cut.
    Plane {
        /// Normal vector to the plane (may not be normalized).
        normal: MathExpr,
        /// Distance of the plane from the origin, or multiple distances.
        distance: MathExpr,
    },
    /// (Hype)rplanar cut that does not pass through the origin.
    PolePlane {
        /// Vector from the origin to the nearest point on the plane, which is
        /// always perpendicular to the plane.
        pole: MathExpr,
    },
    /// Intersection of multiple other cuts.
    Intersection {
        /// Cuts to intersect.
        intersect: Vec<CutSpec>,
    },
}
