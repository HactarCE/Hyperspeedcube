use super::*;

/// Output from cutting an N-dimensional atomic polytope by a slicing plane.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum PolytopeCutOutput {
    /// The polytope is flush with the cutting plane.
    Flush,
    /// The polytope is not flush with the cutting plane.
    NonFlush {
        /// N-dimensional portion of the polytope that is inside the slice, if
        /// any. If this is the whole polytope, then `outside` must be `None`
        /// (but `intersection` may be `Some`). If the inside of the cut is
        /// being deleted, this is `None`.
        inside: Option<PolytopeId>,
        /// N-dimensional portion of the polytope that is outside the slice, if
        /// any. If this is the whole polytope, then `inside` must be `None`
        /// (but `intersection` may be `Some`). If the outside of the cut is
        /// being deleted, this is `None`.
        outside: Option<PolytopeId>,

        /// (N-1)-dimensional intersection of the polytope with the slicing
        /// plane, if any. If `inside` and `outside` are both `Some`, then this
        /// must be `Some`.
        intersection: Option<PolytopeId>,
    },
}
impl fmt::Display for PolytopeCutOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let fmt_option_polytope = |p: Option<PolytopeId>| match p {
            Some(id) => id.to_string(),
            None => "<none>".to_string(),
        };

        match self {
            PolytopeCutOutput::Flush => write!(f, "Flush"),
            PolytopeCutOutput::NonFlush {
                inside,
                outside,
                intersection,
            } => write!(
                f,
                "NonFlush {{ inside: {}, outside: {}, intersection: {} }}",
                fmt_option_polytope(*inside),
                fmt_option_polytope(*outside),
                fmt_option_polytope(*intersection),
            ),
        }
    }
}
impl PolytopeCutOutput {
    /// Result for a polytope that is completely removed by the cut.
    pub const REMOVED: Self = Self::NonFlush {
        inside: None,
        outside: None,
        intersection: None,
    };

    /// Constructs a result for a polytope `p` that is completely inside the
    /// cut.
    pub fn all_inside(p: PolytopeId) -> Self {
        Self::NonFlush {
            inside: Some(p),
            outside: None,
            intersection: None,
        }
    }
    /// Constructs a result for a polytope `p` that is completely outside the
    /// cut.
    pub fn all_outside(p: PolytopeId) -> Self {
        Self::NonFlush {
            inside: None,
            outside: Some(p),
            intersection: None,
        }
    }
    /// Returns an iterator containing `inside` and `outside`, ignoring `None`
    /// values.
    pub fn iter_inside_and_outside(self) -> impl Iterator<Item = PolytopeId> {
        match self {
            PolytopeCutOutput::Flush => itertools::chain(None, None),
            PolytopeCutOutput::NonFlush {
                inside, outside, ..
            } => itertools::chain(inside, outside),
        }
    }
}
