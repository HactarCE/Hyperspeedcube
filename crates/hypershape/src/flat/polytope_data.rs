use super::*;

/// Description of a polytope that is stored in a [`Space`].
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(super) enum PolytopeData {
    /// Vertex (polytope with rank 0).
    Vertex(VertexId),
    /// Polytope with rank greater than 0.
    Polytope {
        /// [Rank] of the polytope.
        ///
        /// [Rank]: https://polytope.miraheze.org/wiki/Rank
        rank: u8,
        /// Facets of the polytope.
        boundary: Set64<ElementId>,
        /// Portals responsible for facets of the polytope.
        ///
        /// In a given [flag] there is at most one element for each portal.
        ///
        /// [flag]: https://polytope.miraheze.org/wiki/Flag
        boundary_portals: BoundaryPortals,
        /// Hyperplane, if the polytope is a facet.
        hyperplane: Option<HyperplaneId>,
        /// Whether the facet is on the surface of the primordial cube. This is
        /// only used for facets.
        is_primordial: bool,
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
    pub fn boundary(&self) -> Result<&Set64<ElementId>> {
        match self {
            PolytopeData::Polytope { boundary, .. } => Ok(boundary),
            _ => bail!("cannot take boundary of rank<2 polytope"),
        }
    }
}

/// List of `(`[`ElementId`], [`PortalId`]`)` pairs.
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
pub(super) struct BoundaryPortals {
    /// Boxed so that it is only a single `usize` when not present.
    entries: Option<Box<SmallVec<[(PortalId, ElementId); 8]>>>,
}

impl BoundaryPortals {
    pub const EMPTY: Self = Self { entries: None };

    pub(super) fn new(iter: impl IntoIterator<Item = (PortalId, ElementId)>) -> Self {
        let mut smallvec: SmallVec<_> = iter.into_iter().collect();
        smallvec.sort_unstable();
        smallvec.dedup();
        Self {
            entries: (!smallvec.is_empty()).then(|| Box::new(smallvec)),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.entries
            .as_ref()
            .is_none_or(|entries| entries.is_empty())
    }

    fn iter(&self) -> impl Iterator<Item = (PortalId, ElementId)> {
        match &self.entries {
            Some(entries) => entries.as_slice(),
            None => &[],
        }
        .iter()
        .copied()
    }

    /// Returns an iterator over portal IDs, in order with no duplicates.
    pub fn iter_portals(&self) -> impl Iterator<Item = PortalId> {
        self.iter().map(|(p, _)| p).dedup()
    }

    /// Returns whether an element is associated with any portal.
    pub fn contains_element(&self, element: ElementId) -> bool {
        self.iter().any(|(_, e)| e == element)
    }

    pub(super) fn pairs_for_element(
        &self,
        element: ElementId,
    ) -> impl Iterator<Item = (PortalId, ElementId)> {
        self.iter().filter(move |&(_, e)| e == element)
    }

    pub fn portals_for_element(&self, element: ElementId) -> impl Iterator<Item = PortalId> {
        self.pairs_for_element(element).map(|(p, _)| p)
    }
}
