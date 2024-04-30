use std::collections::{HashMap, HashSet};

use eyre::{bail, Result};
use hypermath::pga::Blade;
use hypermath::prelude::*;
use itertools::Itertools;

use super::{Simplex, SimplexBlob};
use crate::flat::*;

/// Set of simplices formed from vertices in Euclidean space.
pub struct SimplicialComplex<'space> {
    /// Space in which the vertices of the simplicial complex are defined.
    space: &'space Space,
    /// Set of vertices used by simplices.
    vertices: HashSet<VertexId>,
    /// Set of known simplices.
    cached_simplices: HashMap<PolytopeId, SimplexBlob>,
    /// Set of known centroids of polytopes.
    cached_centroids: HashMap<PolytopeId, Centroid>,
}

impl<'space> SimplicialComplex<'space> {
    /// Constructs a new empty simplicial complex in a space.
    pub fn new(space: &'space Space) -> Self {
        Self {
            space,
            vertices: HashSet::new(),
            cached_simplices: HashMap::new(),
            cached_centroids: HashMap::new(),
        }
    }
    /// Returns the space in which the vertices of the simplicial complex are
    /// defined.
    pub fn space(&self) -> &'space Space {
        self.space
    }

    /// Returns the combined centroid of a set of polytopes, or `None` if the
    /// combined weight is zero. This is only meaningful if all polytopes have
    /// the same rank.
    pub fn combined_centroid(
        &mut self,
        polytopes: impl IntoIterator<Item = PolytopeId>,
    ) -> Result<Option<Centroid>> {
        let mut ret = Centroid::ZERO;
        for p in polytopes {
            ret += self.centroid(p)?;
        }
        Ok((!ret.is_zero()).then_some(ret))
    }

    /// Returns the centroid of a polytope.
    pub fn centroid(&mut self, polytope: PolytopeId) -> Result<Centroid> {
        if let Some(result) = self.cached_centroids.get(&polytope) {
            return Ok(result.clone());
        }

        let ndim = self.space.ndim();

        let mut sum = Centroid::ZERO;
        for simplex in self.simplices(polytope)?.iter() {
            let verts = simplex.vertices();

            // Take the average of the vertices of the simplex.
            let center: Vector = verts.iter().map(|i| &self.space[i]).sum::<Vector>()
                / (simplex.ndim()? + 1) as Float;
            let mut remaining_verts = verts.iter().map(|i| &self.space[i]);
            let Some(init) = remaining_verts.next() else {
                continue;
            };

            // This is scaled by some factor depending on the number of
            // dimensions but that's fine.
            if let Some(blade) = remaining_verts
                .map(|v| Blade::from_vector(ndim, v - init))
                .try_fold(Blade::one(ndim), |a, b| Blade::wedge(&a, &b))
            {
                sum += Centroid::new(&center, blade.mag());
            }
        }
        Ok(sum)
    }

    /// Returns a simplicial complex representing a polytope.
    pub fn simplices(&mut self, polytope: PolytopeId) -> Result<SimplexBlob> {
        if let Some(result) = self.cached_simplices.get(&polytope) {
            return Ok(result.clone());
        }

        let result = match &self.space[polytope] {
            PolytopeData::Vertex(v) => {
                self.vertices.insert(*v);
                Simplex::new([*v]).into()
            }
            PolytopeData::Polytope {
                boundary, flags, ..
            } => {
                if flags.is_primordial {
                    bail!(
                        "primordial cube is present in final shape! \
                         your shape may be infinite",
                    );
                }
                SimplexBlob::from_convex_hull(
                    &boundary
                        .iter()
                        .map(|b| self.simplices(b))
                        .collect::<Result<Vec<_>>>()?,
                )?
            }
        };

        self.cached_simplices.insert(polytope, result.clone());
        Ok(result)
    }

    /// Returns a triangulation of all the polygons in `polytope`.
    pub fn triangles(&mut self, polytope: PolytopeId) -> Result<Vec<[VertexId; 3]>> {
        let polygons = self.space.subelements_with_rank(polytope, 2);
        let simplexes: Vec<SimplexBlob> = polygons
            .iter()
            .map(|polygon| self.simplices(polygon))
            .try_collect()?;
        Ok(simplexes
            .into_iter()
            .flatten()
            .filter_map(|simplex| simplex.try_into_array())
            .collect())
    }
}
