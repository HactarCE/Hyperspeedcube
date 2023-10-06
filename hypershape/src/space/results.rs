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

/// Location of an object (manifold or polytope) relative to the half-spaces on
/// either side of a cut.
///
/// An object could also be a point, but then half of the values for this enum
/// are invalid so instead we use [`PointWhichSide`] for that.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum WhichSide {
    /// The object is flush with the cut. *Every* point on the object is
    /// touching the cut.
    Flush,
    /// The object is inside the cut. It may be touching the cut.
    Inside { is_touching: bool },
    /// The object is entirely outside the cut. It may be touching the cut.
    Outside { is_touching: bool },
    /// The object is split by the cut. It is touching the cut.
    Split,
}
impl Neg for WhichSide {
    type Output = Self;

    fn neg(self) -> Self::Output {
        match self {
            WhichSide::Inside { is_touching } => WhichSide::Outside { is_touching },
            WhichSide::Outside { is_touching } => WhichSide::Inside { is_touching },
            other => other,
        }
    }
}
hypermath::impl_mul_sign!(impl Mul<Sign> for WhichSide);
hypermath::impl_mulassign_sign!(impl MulAssign<Sign> for WhichSide);
impl WhichSide {
    /// Returns whether the manifolds are touching, even at a single point.
    pub fn is_touching(self) -> bool {
        match self {
            WhichSide::Flush => true,
            WhichSide::Inside { is_touching } => is_touching,
            WhichSide::Outside { is_touching } => is_touching,
            WhichSide::Split => true,
        }
    }
    /// Constructs a `WhichSide` from several representative point locations.
    pub(super) fn from_points(points: impl IntoIterator<Item = PointWhichSide>) -> Self {
        let mut is_any_inside = false;
        let mut is_any_outside = false;
        let mut is_touching = false;
        for which_side in points {
            match which_side {
                PointWhichSide::On => is_touching = true,
                PointWhichSide::Inside => is_any_inside = true,
                PointWhichSide::Outside => is_any_outside = true,
            }
        }
        match (is_any_inside, is_any_outside) {
            (true, true) => WhichSide::Split,
            (true, false) => WhichSide::Inside { is_touching },
            (false, true) => WhichSide::Outside { is_touching },
            (false, false) => WhichSide::Flush,
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
