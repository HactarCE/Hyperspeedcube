#![allow(dead_code)] // TODO: remove when extending simplexifier to work with curved cuts

use std::collections::{HashMap, HashSet};
use std::fmt;
use std::ops::Index;

use eyre::{ensure, eyre, Context, OptionExt, Result};
use hypermath::collections::ApproxHashMap;
use hypermath::*;
use hypershape::*;
use itertools::Itertools;
use smallvec::{smallvec, SmallVec};
use tinyset::Set64;

use super::centroid::Centroid;

pub struct Simplexifier<'a> {
    space: &'a Space,

    vertices: Vec<Vector>,
    vertex_ids: ApproxHashMap<Vector, VertexId>,
    shape_simplices_cache: HashMap<AtomicPolytopeId, SimplexBlob>,
}
impl Index<VertexId> for Simplexifier<'_> {
    type Output = Vector;

    fn index(&self, index: VertexId) -> &Self::Output {
        &self.vertices[index.0 as usize]
    }
}
impl<'a> Simplexifier<'a> {
    pub fn new(space: &'a Space) -> Self {
        Simplexifier {
            space,

            vertices: vec![],
            vertex_ids: ApproxHashMap::new(),
            shape_simplices_cache: HashMap::new(),
        }
    }

    fn add_vertex(&mut self, p: Point) -> Result<VertexId> {
        let v = p.to_finite().map_err(|_| eyre!("infinite point"))?;
        Ok(*self.vertex_ids.entry(v.clone()).or_insert_with(|| {
            let id = VertexId(self.vertices.len() as u32);
            self.vertices.push(v);
            id
        }))
    }

    pub fn shape_centroid_point(&mut self, shape: AtomicPolytopeId) -> Result<Vector> {
        let shape_centroid = self.shape_centroid(shape)?;
        self.project_point_onto_shape(&Point::Finite(shape_centroid.center()), shape)
            .wrap_err("unable to compute centroid of shape")
    }
    pub fn project_point_onto_shape(
        &self,
        point: &Point,
        shape: AtomicPolytopeId,
    ) -> Result<Vector> {
        let manifold = self.space[shape].manifold;
        let blade = &self.space[manifold].blade;
        // Project the point back onto the manifold.
        blade
            .project_point(point)
            .and_then(|p| p.to_finite().ok())
            .ok_or_eyre("unable to project centroid of shape")
    }
    pub fn shape_centroid(&mut self, shape: AtomicPolytopeId) -> Result<Centroid> {
        let shape_manifold = self.space[shape].manifold;
        // Turn the shape into simplices.
        let simplices = self.shape_simplices(shape)?.0.into_iter();
        // Compute the centroid of each simplex.
        let centroids = simplices.map(|s| self.simplex_centroid(&s, shape_manifold));
        // Add up those centroids.
        centroids.sum::<Result<Centroid>>()
    }

    fn simplex_centroid(&self, s: &Simplex, m: ManifoldId) -> Result<Centroid> {
        let mut verts_iter = s.0.iter();
        let Some(v0) = verts_iter.next() else {
            return Ok(Centroid::default());
        };
        let center = self.simplex_center(s, m)?;
        let weight = verts_iter
            .fold(cga::Blade::scalar(1.0), |b, v| {
                b ^ cga::Blade::vector(&self[v] - &self[v0])
            })
            .abs_mag2();
        Ok(Centroid::new(&center, weight))
    }
    fn simplex_center(&self, s: &Simplex, m: ManifoldId) -> Result<Vector> {
        let mut sum = Vector::EMPTY;
        for v in s.0.iter() {
            sum += &self[v];
        }
        let point = sum / s.0.len() as Float;
        let blade = &self.space[m].blade;
        if blade.opns_is_flat() {
            Ok(point)
        } else {
            blade
                .project_point(&cga::Point::Finite(point))
                .and_then(|p| p.to_finite().ok())
                .ok_or_eyre("failed to project point onto manifold")
        }
    }

    fn shape_simplices(&mut self, shape: AtomicPolytopeId) -> Result<SimplexBlob> {
        match self.shape_simplices_cache.get(&shape) {
            Some(cached) => Ok(cached.clone()),
            None => {
                let ret = self.shape_simplices_uncached(shape)?;
                self.shape_simplices_cache.insert(shape, ret.clone());
                Ok(ret)
            }
        }
    }
    fn shape_simplices_uncached(&mut self, shape: AtomicPolytopeId) -> Result<SimplexBlob> {
        let manifold = self.space[shape].manifold;
        let blade = &self.space[manifold].blade;

        ensure!(
            blade.opns_is_flat(),
            "spherical shapes are not yet supported",
        );

        if self.space.ndim_of(manifold) == 1 {
            let edge = self.space[shape]
                .boundary
                .iter()
                .exactly_one()
                .map_err(|_| eyre!("edge has multiple boundary elements"))?;
            let [a, b] = self.space.extract_point_pair(edge)?;
            let a = self.add_vertex(a)?;
            let b = self.add_vertex(b)?;
            Ok(SimplexBlob::new([Simplex::new([a, b])]))
        } else {
            let boundary_simplices = self.space[shape]
                .boundary
                .iter()
                .map(|boundary_elem| self.shape_simplices(boundary_elem.id))
                .collect::<Result<Vec<SimplexBlob>>>()?;
            SimplexBlob::from_convex_hull(&boundary_simplices)
        }
    }

    pub fn polygon_edges(&mut self, polygon: AtomicPolytopeRef) -> Result<Vec<[VertexId; 2]>> {
        let manifold = self.space.manifold_of(polygon);
        let blade = self.space.blade_of(manifold);

        ensure!(
            self.space.ndim_of(manifold) == 2,
            "cannot triangulate non-polygon",
        );

        let is_flat = blade.opns_is_flat();
        let boundary_is_flat = self
            .space
            .boundary_of(polygon)
            .all(|b| self.space[self.space[b.id].manifold].blade.opns_is_flat());
        ensure!(
            is_flat && boundary_is_flat,
            "spherical shapes are not yet supported",
        );

        self.space
            .boundary_of(polygon)
            .map(|edge| {
                let edge_bounds =
                    self.space.boundary_of(edge).exactly_one().map_err(|e| {
                        eyre!("edge should be bounded by exactly one point pair: {e}")
                    })?;
                let [a, b] = self.space.extract_point_pair(edge_bounds)?;
                let a = self.add_vertex(a)?;
                let b = self.add_vertex(b)?;
                Ok([a, b])
            })
            .collect()
    }
    pub fn triangles(&mut self, polygon: AtomicPolytopeRef) -> Result<Vec<[VertexId; 3]>> {
        let edges = self.polygon_edges(polygon)?;
        let initial_vertex = edges.first().ok_or_eyre("polygon has no edges")?[0];
        Ok(edges
            .into_iter()
            .filter(|edge| !edge.contains(&initial_vertex))
            .map(|[a, b]| [initial_vertex, a, b])
            .collect())
    }

    pub fn vertex_set(&mut self, root: AtomicPolytopeRef) -> Result<HashSet<VertexId>> {
        self.space
            .children_with_ndim(root, 2)
            .into_iter()
            .map(|polygon| self.polygon_edges(polygon))
            .flatten_ok()
            .flatten_ok()
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct Simplex(Set64<VertexId>);
impl fmt::Display for Simplex {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Simplex({})", self.0.iter().join(", "))
    }
}
impl Simplex {
    fn new(verts: impl IntoIterator<Item = VertexId>) -> Self {
        Simplex(verts.into_iter().collect())
    }
    fn ndim(&self) -> Result<u8> {
        (self.0.len() as u8)
            .checked_sub(1)
            .ok_or_eyre("simplex cannot be empty")
    }
    fn try_into_array<const N: usize>(&self) -> Option<[VertexId; N]> {
        self.0.iter().collect_vec().try_into().ok()
    }
    fn arbitrary_vertex(&self) -> Result<VertexId> {
        self.0.iter().next().ok_or_eyre("simplex is empty")
    }
    /// Returns all 1-dimensional elemenst of the simplex.
    fn edges(&self) -> impl '_ + Iterator<Item = [VertexId; 2]> {
        let verts: SmallVec<[VertexId; 8]> = self.0.iter().collect();
        verts
            .into_iter()
            .tuple_combinations()
            .map(|(v1, v2)| [v1, v2])
    }
    /// Returns all (N-1)-dimensional elements of the simplex.
    fn facets(&self) -> Result<impl '_ + Iterator<Item = Simplex>> {
        let ndim = self.ndim()?;
        let facet_ndim = ndim.checked_sub(1).ok_or_eyre("0D simplex has no facets")?;
        Ok(self.elements(facet_ndim))
    }
    /// Returns all elements of the simplex with a given number of dimensions.
    fn elements(&self, ndim: u8) -> impl '_ + Iterator<Item = Simplex> {
        self.0
            .iter()
            .combinations(ndim as usize + 1)
            .map(|verts| Simplex(Set64::from_iter(verts)))
    }
}

/// Convex polytope made of simplices.
#[derive(Debug, Default, Clone)]
struct SimplexBlob(SmallVec<[Simplex; 2]>);
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
impl SimplexBlob {
    const EMPTY: Self = SimplexBlob(SmallVec::new_const());

    fn new(simplices: impl IntoIterator<Item = Simplex>) -> Self {
        SimplexBlob(simplices.into_iter().collect())
    }

    fn from_convex_hull(facets: &[SimplexBlob]) -> Result<Self> {
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
        let vertex_set: Set64<VertexId> = facet_simplices.flat_map(|s| s.0.iter()).collect();

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
            // Pick a vertex to start from. This `.unwrap()` always succeeds
            // because `.ndim()` succeded.
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

    fn extend(&mut self, other: SimplexBlob) {
        self.0.extend(other.0);
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct VertexId(u32);
impl fmt::Display for VertexId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "#{}", self.0)
    }
}
impl tinyset::Fits64 for VertexId {
    unsafe fn from_u64(x: u64) -> Self {
        Self(x as u32)
    }

    fn to_u64(self) -> u64 {
        self.0 as u64
    }
}
