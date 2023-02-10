use anyhow::{anyhow, Result};
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
    /// (Hyper)planar cut that does not pass through the origin.
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

#[derive(Debug, Clone)]
pub(crate) struct FlattenedCutSpec<'a> {
    /// Cut determined by a mathematical expression.
    pub cut: &'a Option<CutSpec>,
    /// Center of a (hyper)spherical cut.
    pub center: &'a Option<MathExpr>,
    /// Radius of a (hyper)spherical cut, or multiple radii.
    pub radius: &'a Option<MathExpr>,
    /// Normal vector to a (hyper)planar cut (may not be normalized).
    pub normal: &'a Option<MathExpr>,
    /// Distance of a (hyper)planar cut from the origin, or multiple distances.
    pub distance: &'a Option<MathExpr>,
    /// Vector from the origin to the nearest point on the (hyper)planar cut, which is
    /// always perpendicular to the (hyper)plane.
    pub pole: &'a Option<MathExpr>,
    /// Cuts to intersect.
    pub intersect: &'a Option<Vec<CutSpec>>,
}

impl<'a> TryFrom<FlattenedCutSpec<'a>> for CutSpec {
    type Error = anyhow::Error;

    fn try_from(value: FlattenedCutSpec<'a>) -> Result<Self> {
        let FlattenedCutSpec {
            cut,
            center,
            radius,
            normal,
            distance,
            pole,
            intersect,
        } = value;

        let total = cut.is_some() as u8
            + center.is_some() as u8
            + radius.is_some() as u8
            + normal.is_some() as u8
            + distance.is_some() as u8
            + pole.is_some() as u8
            + intersect.is_some() as u8;
        if let (1, Some(cut)) = (total, cut) {
            Ok(cut.clone())
        } else if let (2, Some(center), Some(radius)) = (total, center.clone(), radius.clone()) {
            Ok(CutSpec::Sphere { center, radius })
        } else if let (2, Some(normal), Some(distance)) = (total, normal.clone(), distance.clone())
        {
            Ok(CutSpec::Plane { normal, distance })
        } else if let (1, Some(pole)) = (total, pole.clone()) {
            Ok(CutSpec::PolePlane { pole })
        } else if let (1, Some(intersect)) = (total, intersect.clone()) {
            Ok(CutSpec::Intersection { intersect })
        } else {
            Err(anyhow!("invalid cut: {:?}", value))
        }
    }
}
