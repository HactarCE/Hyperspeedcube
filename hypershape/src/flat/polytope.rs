use super::*;

/// Description of a polytope that is stored in a [`Space`].
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PolytopeData {
    /// Vertex (polytope with rank 0).
    ///
    /// This is somewhat redundant with just using `VertexId` directly, but it's
    /// often handy to avoid a special case for 0-dimensional or 1-dimensional
    /// polytopes.
    Vertex(VertexId),
    /// Polytope with rank greater than 0.
    Polytope {
        /// [Rank] of the polytope.
        ///
        /// [Rank]: https://polytope.miraheze.org/wiki/Rank
        rank: u8,
        /// Facets of the polytope.
        boundary: PolytopeSet,
        /// Extra data about the polytope.
        flags: PolytopeFlags,
    },
}
impl From<VertexId> for PolytopeData {
    fn from(value: VertexId) -> Self {
        PolytopeData::Vertex(value)
    }
}
impl PolytopeData {
    /// Returns the [rank] of the polytope.
    ///
    /// [rank]: https://polytope.miraheze.org/wiki/Rank
    pub fn rank(&self) -> u8 {
        match self {
            PolytopeData::Vertex(_) => 0,
            PolytopeData::Polytope { rank, .. } => *rank,
        }
    }

    /// Returns the polytope as a single vertex, or `None` if the polytope is
    /// not a vertex.
    pub fn to_vertex(&self) -> Option<VertexId> {
        match self {
            PolytopeData::Vertex(v) => Some(*v),
            _ => None,
        }
    }

    /// Returns the set of boundary elements, or an error if the polytope has
    /// rank less than 2.
    pub fn boundary(&self) -> Result<&PolytopeSet> {
        match self {
            PolytopeData::Polytope { boundary, .. } => Ok(boundary),
            _ => bail!("cannot take boundary of rank<2 polytope"),
        }
    }
}

/// Extra data about a polytope.
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, Hash)]
pub struct PolytopeFlags {
    /// Whether the polytope is on the surface of the primordial cube.
    pub is_primordial: bool,
}
