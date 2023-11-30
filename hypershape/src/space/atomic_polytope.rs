use super::*;

/// Conformally convex polytope defined by a manifold and a boundary.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct AtomicPolytope {
    /// Manifold of the polytope.
    pub manifold: ManifoldId,
    /// Boundary of the polytope, represented using a set of atomic polytopes of
    /// one dimension lower.
    pub boundary: AtomicPolytopeSet,
}

impl fmt::Debug for AtomicPolytope {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let manifold_str = self.manifold.to_string();
        let boundary_str = self
            .boundary
            .iter()
            .map(|boundary_elem| format!("{boundary_elem:?}"))
            .join(", ");
        write!(
            f,
            "AtomicPolytope {{ manifold: {}, boundary: [{}] }}",
            manifold_str, boundary_str,
        )
    }
}

impl AtomicPolytope {
    /// Constructs an atomic polytope.
    pub fn new(manifold: ManifoldId, boundary: AtomicPolytopeSet) -> Self {
        Self { manifold, boundary }
    }
    /// Constructs an atomic polytope with no boundary.
    pub fn whole_manifold(manifold: ManifoldId) -> Self {
        Self::new(manifold, Set64::new())
    }
}

impl Mul<Sign> for AtomicPolytopeId {
    type Output = AtomicPolytopeRef;

    fn mul(self, rhs: Sign) -> Self::Output {
        AtomicPolytopeRef {
            id: self,
            sign: rhs,
        }
    }
}
