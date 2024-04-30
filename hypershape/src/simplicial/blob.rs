use std::fmt;

use eyre::{ensure, Result};
use itertools::Itertools;
use smallvec::{smallvec, SmallVec};
use tinyset::Set64;

use super::Simplex;
use crate::flat::*;

/// Simplexes comprising a convex polytope.
#[derive(Debug, Default, Clone)]
pub struct SimplexBlob(SmallVec<[Simplex; 2]>);

impl fmt::Display for SimplexBlob {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Blob[{}]", self.0.iter().join(", "))
    }
}

impl From<Simplex> for SimplexBlob {
    fn from(s: Simplex) -> Self {
        SimplexBlob::new([s])
    }
}

impl IntoIterator for SimplexBlob {
    type Item = Simplex;

    type IntoIter = smallvec::IntoIter<[Simplex; 2]>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl SimplexBlob {
    /// Empty simplex blob.
    pub const EMPTY: Self = SimplexBlob(SmallVec::new_const());

    /// Constructs a `SimplexBlob` from a list of simplices.
    pub fn new(simplices: impl IntoIterator<Item = Simplex>) -> Self {
        SimplexBlob(simplices.into_iter().collect())
    }

    /// Constructs a higher-dimensional simplex blob from a list of facets. No
    /// validation is done on the facets.
    pub fn from_convex_hull(facets: &[SimplexBlob]) -> Result<Self> {
        let Some(arbitrary_facet) = facets.iter().find_map(|f| f.0.first()) else {
            return Ok(SimplexBlob::EMPTY);
        };
        let facet_ndim = arbitrary_facet.ndim()?;

        ensure!(
            facets
                .iter()
                .flat_map(|f| &f.0)
                .all(|s| s.ndim().ok() == Some(facet_ndim)),
            "cannot construct simplex blob from \
             dimension-mismatched convex hull",
        );

        let facet_simplices = facets.iter().flat_map(|f| &f.0);
        let vertex_set: Set64<VertexId> =
            facet_simplices.flat_map(|s| s.vertices().iter()).collect();

        // Optimization: if the number of simplices equals the facet dimension
        // plus 2 equals the nubmer of vertices, then the result is a single
        // simplex.
        let number_of_simplices = facets.iter().map(|f| f.0.len()).sum::<usize>();
        let is_single_simplex = number_of_simplices == facet_ndim as usize + 2
            && number_of_simplices == vertex_set.len();
        if is_single_simplex {
            // Construct the single simplex.
            Ok(SimplexBlob::new([Simplex(vertex_set)]))
        } else {
            // Pick a vertex to start from.
            let initial_vertex = arbitrary_facet.arbitrary_vertex()?;
            Ok(SimplexBlob::from_convex_hull_and_initial_vertex(
                facets,
                initial_vertex,
            ))
        }
    }

    fn from_convex_hull_and_initial_vertex(
        facets: &[SimplexBlob],
        initial_vertex: VertexId,
    ) -> Self {
        let mut ret = smallvec![];

        // For every facet that does not contain that vertex ...
        for facet in facets {
            if facet.0.iter().all(|s| !s.0.contains(initial_vertex)) {
                // ... for every simplex in that facet ...
                for simplex in &facet.0 {
                    // ... construct a new simplex that will be in the result.
                    let mut simplex = simplex.clone();
                    simplex.0.insert(initial_vertex);
                    // And add that simplex, if it's not a duplicate.
                    if !ret.contains(&simplex) {
                        ret.push(simplex);
                    }
                }
            }
        }

        SimplexBlob(ret)
    }

    /// Merges another blob of simplexes into this one.
    pub fn extend(&mut self, other: SimplexBlob) {
        self.0.extend(other.0);
    }

    /// Iterates over simplices in the blob.
    pub fn iter(&self) -> impl '_ + Iterator<Item = &Simplex> {
        self.0.iter()
    }
}
