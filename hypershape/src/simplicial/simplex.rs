use std::fmt;

use eyre::{OptionExt, Result};
use itertools::Itertools;
use smallvec::SmallVec;
use tinyset::Set64;

use crate::flat::*;

/// Simplex represented as a set of vertices.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Simplex(pub Set64<VertexId>);

impl fmt::Display for Simplex {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Simplex({})", self.0.iter().join(", "))
    }
}

impl Simplex {
    /// Constructs a simplex from vertices.
    pub fn new(verts: impl IntoIterator<Item = VertexId>) -> Self {
        Simplex(verts.into_iter().collect())
    }

    /// Returns the number of dimensions of the simplex, which is inferred from
    /// the number of vertices.
    pub fn ndim(&self) -> Result<u8> {
        (self.0.len() as u8)
            .checked_sub(1)
            .ok_or_eyre("simplex cannot be empty")
    }
    /// Returns the vertex set of the simplex.
    pub fn vertices(&self) -> &Set64<VertexId> {
        &self.0
    }

    /// Converts the simplex into an array of `N` vertices, or returns `None` if
    /// the simplex has the wrong number of vertices.
    pub fn try_into_array<const N: usize>(&self) -> Option<[VertexId; N]> {
        self.0.iter().collect_vec().try_into().ok()
    }

    /// Returns a vertex from the simplex, or an error if the simplex is empty.
    pub(super) fn arbitrary_vertex(&self) -> Result<VertexId> {
        self.0.iter().next().ok_or_eyre("simplex is empty")
    }

    /// Returns all 1-dimensional elements of the simplex.
    pub fn edges(&self) -> impl '_ + Iterator<Item = [VertexId; 2]> {
        let verts: SmallVec<[VertexId; 8]> = self.0.iter().collect();
        verts
            .into_iter()
            .tuple_combinations()
            .map(|(v1, v2)| [v1, v2])
    }
    /// Returns all (N-1)-dimensional elements of the simplex.
    pub fn facets(&self) -> Result<impl '_ + Iterator<Item = Simplex>> {
        let ndim = self.ndim()?;
        let facet_ndim = ndim.checked_sub(1).ok_or_eyre("0D simplex has no facets")?;
        Ok(self.elements(facet_ndim))
    }
    /// Returns all elements of the simplex with a given number of dimensions.
    pub fn elements(&self, ndim: u8) -> impl '_ + Iterator<Item = Simplex> {
        self.0
            .iter()
            .combinations(ndim as usize + 1)
            .map(|verts| Simplex(Set64::from_iter(verts)))
    }
}
