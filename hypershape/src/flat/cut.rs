use super::*;

/// Parameters for cutting shapes.
#[derive(Clone)]
pub struct CutParams {
    /// Plane that divides the inside of the cut from the outside of the cut.
    pub divider: Hyperplane,
    /// What to do with the shapes on the inside of the cut.
    pub inside: PolytopeFate,
    /// What to do with the shapes on the outside of the cut.
    pub outside: PolytopeFate,
}
impl fmt::Debug for CutParams {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{{ divider: {:?}, inside: {}, outside: {} }}",
            self.divider, self.inside, self.outside,
        )
    }
}

/// What to do with a shape resulting from a cutting operation.
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, Hash)]
pub enum PolytopeFate {
    /// The shape should be removed.
    #[default]
    Remove,
    /// The shape should remain.
    Keep,
}
impl fmt::Display for PolytopeFate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PolytopeFate::Remove => write!(f, "REMOVE"),
            PolytopeFate::Keep => write!(f, "KEEP"),
        }
    }
}

/// In-progress cut operation, which caches intermediate results.
#[derive(Debug)]
pub struct Cut {
    /// Cut parameters.
    pub(super) params: CutParams,
    /// Cache of the result of splitting each shape.
    pub(super) polytope_cut_output_cache: HashMap<PolytopeId, PolytopeCutOutput>,
}
impl fmt::Display for Cut {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Cut")
            .field("params", &self.params)
            .finish_non_exhaustive()
    }
}
impl Cut {
    /// Constructs a cutting operation that deletes polytopes on the outside of
    /// the cut and keeps only those on the inside.
    pub fn carve(divider: Hyperplane) -> Self {
        Self::new(CutParams {
            divider,
            inside: PolytopeFate::Keep,
            outside: PolytopeFate::Remove,
        })
    }
    /// Constructs a cutting operation that keeps all resulting polytopes.
    pub fn slice(divider: Hyperplane) -> Self {
        Self::new(CutParams {
            divider,
            inside: PolytopeFate::Keep,
            outside: PolytopeFate::Keep,
        })
    }

    /// Constructs a cutting operation.
    pub fn new(params: CutParams) -> Self {
        Self {
            params,
            polytope_cut_output_cache: HashMap::new(),
        }
    }

    /// Returns the parameters used to create the cut.
    pub fn params(&self) -> &CutParams {
        &self.params
    }
}
