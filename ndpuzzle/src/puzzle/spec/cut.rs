use serde::{Deserialize, Serialize};

use super::MathExpr;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(untagged, deny_unknown_fields)]
pub enum CutSpec {
    Expr(MathExpr),
    Sphere {
        center: MathExpr,
        radius: MathExpr,
    },
    Plane {
        normal: MathExpr,
        distance: MathExpr,
    },
    PolePlane {
        /// Vector from the origin to the facet plane and perpendicular to the
        /// facet.
        pole: MathExpr,
    },
    Intersection {
        intersect: Vec<CutSpec>,
    },
}
