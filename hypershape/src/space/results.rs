use super::*;

/// Union of two intervals on a topological circle (1D manifold).
pub(super) enum IntervalUnion {
    /// Union of the two intervals.
    Union(AtomicPolytopeRef),
    /// The union of the two intervals is the whole space.
    WholeSpace,
    /// The two intervals do not intersect, and therefore their union is
    /// disconnected.
    Disconnected,
}

/// Location of one manifold relative to the half-spaces on either side of
/// another cut.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub(super) enum ManifoldWhichSide {
    /// The manifold is flush with the cut.
    Flush,
    /// The manifold is entirely inside the cut. It may be tangent at a single
    /// point.
    Inside,
    /// The manifold is entirely outside the cut. It may be tangent at a single
    /// point.
    Outside,
    /// The manifold is split by the cut.
    Split,
}
impl Neg for ManifoldWhichSide {
    type Output = Self;

    fn neg(self) -> Self::Output {
        match self {
            ManifoldWhichSide::Inside => ManifoldWhichSide::Outside,
            ManifoldWhichSide::Outside => ManifoldWhichSide::Inside,
            other => other,
        }
    }
}
hypermath::impl_mul_sign!(impl Mul<Sign> for ManifoldWhichSide);
hypermath::impl_mulassign_sign!(impl MulAssign<Sign> for ManifoldWhichSide);
impl ManifoldWhichSide {
    pub fn from_points(points: impl IntoIterator<Item = PointWhichSide>) -> Self {
        let mut is_any_inside = false;
        let mut is_any_outside = false;
        for which_side in points {
            match which_side {
                PointWhichSide::On => (),
                PointWhichSide::Inside => is_any_inside = true,
                PointWhichSide::Outside => is_any_outside = true,
            }
        }
        match (is_any_inside, is_any_outside) {
            (true, true) => ManifoldWhichSide::Split,
            (true, false) => ManifoldWhichSide::Inside,
            (false, true) => ManifoldWhichSide::Outside,
            (false, false) => ManifoldWhichSide::Flush,
        }
    }
}

/// Output from cutting an N-dimensional atomic polytope by a slicing manifold.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum AtomicPolytopeCutOutput {
    /// The polytope's manifold is flush with the slicing manifold.
    Flush,
    /// The polytope's manifold is completely on the inside of the slice.
    ManifoldInside,
    /// The polytope's manifold is completely on the outside of the slice.
    ManifoldOutside,
    /// The polytope's manifold intersects the slice but is not flush.
    NonFlush {
        /// N-dimensional portion of the polytope that is inside the slice, if
        /// any. If this is the whole polytope, then `outside` must be `None`
        /// (but `intersection` may be `Some`). If the inside of the cut is
        /// being deleted, this is `None`.
        inside: Option<AtomicPolytopeRef>,
        /// N-dimensional portion of the polytope that is outside the slice, if
        /// any. If this is the whole polytope, then `inside` must be `None`
        /// (but `intersection` may be `Some`). If the outside of the cut is
        /// being deleted, this is `None`.
        outside: Option<AtomicPolytopeRef>,

        /// (N-1)-dimensional intersection of the polytope with the slicing
        /// manifold, if any. If `inside` and `outside` are both `Some`, then
        /// this must be `Some`.
        intersection: Option<AtomicPolytopeRef>,
    },
}
impl fmt::Display for AtomicPolytopeCutOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AtomicPolytopeCutOutput::Flush => write!(f, "Flush"),
            AtomicPolytopeCutOutput::ManifoldInside => write!(f, "ManifoldInside"),
            AtomicPolytopeCutOutput::ManifoldOutside => write!(f, "ManifoldOutside"),
            AtomicPolytopeCutOutput::NonFlush {
                inside,
                outside,
                intersection: intersection_shape,
            } => {
                write!(
                    f,
                    "NonFlush {{ inside: {}, outside: {}, intersection_shape: {} }}",
                    inside.map_or_else(|| "<none>".to_string(), |x| x.to_string()),
                    outside.map_or_else(|| "<none>".to_string(), |x| x.to_string()),
                    intersection_shape.map_or_else(|| "<none>".to_string(), |x| x.to_string()),
                )
            }
        }
    }
}
impl Neg for AtomicPolytopeCutOutput {
    type Output = Self;

    fn neg(mut self) -> Self::Output {
        fn negate_option_shape_ref(r: &mut Option<AtomicPolytopeRef>) {
            if let Some(r) = r {
                *r = -*r;
            }
        }

        if let AtomicPolytopeCutOutput::NonFlush {
            inside,
            outside,
            intersection: intersection_shape,
        } = &mut self
        {
            negate_option_shape_ref(inside);
            negate_option_shape_ref(outside);
            negate_option_shape_ref(intersection_shape);
        }

        self
    }
}

hypermath::impl_mul_sign!(impl Mul<Sign> for AtomicPolytopeCutOutput);
hypermath::impl_mulassign_sign!(impl MulAssign<Sign> for AtomicPolytopeCutOutput);
