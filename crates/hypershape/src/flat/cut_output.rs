use super::*;

/// Output from cutting an N-dimensional polytope element by a slicing plane.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum ElementCutOutput {
    /// The element is flush with the cutting plane.
    Flush,
    /// The element is not flush with the cutting plane.
    NonFlush {
        /// N-dimensional portion of the element that is inside the slice, if
        /// any. If this is the whole element, then `outside` must be `None`
        /// (but `intersection` may be `Some`). If the inside of the cut is
        /// being deleted, this is `None`.
        inside: Option<ElementId>,
        /// N-dimensional portion of the element that is outside the slice, if
        /// any. If this is the whole element, then `inside` must be `None` (but
        /// `intersection` may be `Some`). If the outside of the cut is being
        /// deleted, this is `None`.
        outside: Option<ElementId>,

        /// (N-1)-dimensional intersection of the element with the slicing
        /// plane, if any. If `inside` and `outside` are both `Some`, then this
        /// must be `Some`.
        intersection: Option<ElementId>,
    },
}
impl fmt::Display for ElementCutOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let fmt_option_polytope = |p: Option<ElementId>| match p {
            Some(id) => id.to_string(),
            None => "<none>".to_string(),
        };

        match self {
            ElementCutOutput::Flush => write!(f, "Flush"),
            ElementCutOutput::NonFlush {
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
impl ElementCutOutput {
    /// Result for an element that is completely removed by the cut.
    pub const REMOVED: Self = Self::NonFlush {
        inside: None,
        outside: None,
        intersection: None,
    };

    /// Constructs a result for an element `p` that is completely on one side of
    /// the cut.
    pub fn all_same(which: PointWhichSide, p: ElementId, intersection: Option<ElementId>) -> Self {
        match which {
            PointWhichSide::On => Self::Flush,
            PointWhichSide::Inside => Self::all_inside(p, intersection),
            PointWhichSide::Outside => Self::all_outside(p, intersection),
        }
    }
    /// Constructs a result for an element `p` that is completely inside the
    /// cut.
    pub fn all_inside(p: ElementId, intersection: Option<ElementId>) -> Self {
        Self::NonFlush {
            inside: Some(p),
            outside: None,
            intersection,
        }
    }
    /// Constructs a result for an element `p` that is completely outside the
    /// cut.
    pub fn all_outside(p: ElementId, intersection: Option<ElementId>) -> Self {
        Self::NonFlush {
            inside: None,
            outside: Some(p),
            intersection,
        }
    }

    /// Returns whether the element `original` that was cut was unchanged by the
    /// cut. It may still have had a facet that was flush with the cutting
    /// plane.
    pub fn is_unchanged_from(self, original: ElementId) -> bool {
        self.iter_inside_and_outside().eq([original])
    }

    /// Returns the portion of the element on the inside of the cut.
    pub fn inside(self) -> Option<ElementId> {
        match self {
            ElementCutOutput::Flush => None,
            ElementCutOutput::NonFlush { inside, .. } => inside,
        }
    }
    /// Returns the portion of the element on the outside of the cut.
    pub fn outside(self) -> Option<ElementId> {
        match self {
            ElementCutOutput::Flush => None,
            ElementCutOutput::NonFlush { outside, .. } => outside,
        }
    }

    /// Returns an iterator containing `inside` and `outside`, ignoring `None`
    /// values.
    pub fn iter_inside_and_outside(self) -> impl Iterator<Item = ElementId> {
        match self {
            ElementCutOutput::Flush => itertools::chain(None, None),
            ElementCutOutput::NonFlush {
                inside, outside, ..
            } => itertools::chain(inside, outside),
        }
    }
}
