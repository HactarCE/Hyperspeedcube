use super::*;

/// Reference to an element of any rank.
pub type Element<'a> = SpaceRef<'a, ElementId>;
/// Reference to a polytope (max-rank element).
pub type Polytope<'a> = SpaceRef<'a, PolytopeId>;
/// Reference to a facet (one-less-than-max-rank element).
pub type Facet<'a> = SpaceRef<'a, FacetId>;
/// Reference to a face (rank-2 element).
pub type Face<'a> = SpaceRef<'a, FaceId>;
/// Reference to an edge (rank-1 element).
pub type Edge<'a> = SpaceRef<'a, EdgeId>;
/// Reference to a vertex (rank-0 element).
pub type Vertex<'a> = SpaceRef<'a, VertexId>;

/// Set of elements of any rank.
pub type ElementSet<'a> = SpaceRef<'a, Set64<ElementId>>;
/// Set of polytopes (max-rank elements).
pub type PolytopeSet<'a> = SpaceRef<'a, Set64<PolytopeId>>;
/// Set of facets (one-less-than-max-rank elements).
pub type FacetSet<'a> = SpaceRef<'a, Set64<FacetId>>;
/// Set of faces (rank-2 elements).
pub type FaceSet<'a> = SpaceRef<'a, Set64<FaceId>>;
/// Set of edges (rank-1 elements).
pub type EdgeSet<'a> = SpaceRef<'a, Set64<EdgeId>>;
/// Set of vertices (rank-0 elements).
pub type VertexSet<'a> = SpaceRef<'a, Set64<VertexId>>;

/// Trait for types that reference an element of a polytope.
pub trait ToElementId: Copy {
    /// Returns the ID of the corresponding polytope.
    fn to_element_id(&self, space: &Space) -> ElementId;
}
impl ToElementId for ElementId {
    fn to_element_id(&self, _space: &Space) -> ElementId {
        *self
    }
}
impl ToElementId for PolytopeId {
    fn to_element_id(&self, _space: &Space) -> ElementId {
        ElementId(self.0)
    }
}
impl ToElementId for FacetId {
    fn to_element_id(&self, _space: &Space) -> ElementId {
        ElementId(self.0)
    }
}
impl ToElementId for FaceId {
    fn to_element_id(&self, _space: &Space) -> ElementId {
        ElementId(self.0)
    }
}
impl ToElementId for EdgeId {
    fn to_element_id(&self, _space: &Space) -> ElementId {
        ElementId(self.0)
    }
}
impl ToElementId for VertexId {
    fn to_element_id(&self, space: &Space) -> ElementId {
        space.vertex_to_polytope(*self)
    }
}

impl<'a, I: ToElementId> SpaceRef<'a, I> {
    /// Returns a reference to the element as an element, ignoring any
    /// information about its rank.
    pub fn as_element(self) -> Element<'a> {
        Element {
            space: self.space,
            id: self.to_element_id(self.space),
        }
    }
    /// Returns an iterator over the subelements of all ranks.
    pub fn subelements(self) -> ElementIter<'a> {
        ElementIter::new(self.space, self.space.subelements_of(self.as_element().id))
    }
    /// Returns an iterator over the subelements with a specific rank, or an
    /// empty set if `rank` is invalid.
    pub fn subelements_with_rank(self, rank: u8) -> ElementIter<'a> {
        let iter = self.space.subelements_with_rank(self.as_element().id, rank);
        ElementIter::new(self.space, iter)
    }
    /// Returns an iterator over the faces of the element.
    pub fn face_set(self) -> MappedElementIter<'a, FaceId> {
        self.subelements_with_rank(2)
            .map(|e| e.map_id(|id| FaceId(id.0)))
    }
    /// Returns an iterator over the edges of the element.
    pub fn edge_set(self) -> MappedElementIter<'a, EdgeId> {
        self.subelements_with_rank(1)
            .map(|e| e.map_id(|id| EdgeId(id.0)))
    }
    /// Returns an iterator over the set of unique vertices of the element.
    pub fn vertex_set(self) -> impl Iterator<Item = Vertex<'a>> {
        let space = self.space;
        space
            .vertex_set(self.as_element().id)
            .into_iter()
            .map(|v| space.get(v))
    }

    /// Returns the centroid of the element.
    pub fn centroid(self) -> Result<Centroid> {
        self.space.centroid(self.as_element().id)
    }

    /// Returns a decomposition of the element into simplices.
    pub fn simplices(self) -> Result<SimplexBlob> {
        self.space.simplices(self.as_element().id)
    }
    /// Returns a decomposition of the faces of the element into triangles.
    pub fn triangles(self) -> Result<Vec<[VertexId; 3]>> {
        self.space.triangles(self.as_element().id)
    }
    /// Returns an arbitrary vertex of the element.
    pub fn arbitrary_vertex(self) -> Result<Vertex<'a>> {
        self.vertex_set().next().ok_or_eyre("degenerate polytope")
    }

    /// Returns which side of `divider` contains the element.
    pub fn is_on_which_side_of(self, divider: &Hyperplane) -> WhichSide {
        self.space
            .which_side_has_polytope(divider, self.as_element().id)
    }
}
impl<I: ToElementId> ToElementId for SpaceRef<'_, I> {
    fn to_element_id(&self, space: &Space) -> ElementId {
        space.ensure_same_as(self.space).expect("different space");
        self.id.to_element_id(space)
    }
}
impl<'a, I: Fits64> SpaceRef<'a, Set64<I>> {
    /// Returns whether the set is empty.
    pub fn is_empty(&self) -> bool {
        self.id.is_empty()
    }
    /// Returns the number of elements in the set.
    pub fn len(&self) -> usize {
        self.id.len()
    }
    /// Returns an iterator over the elements of the set.
    pub fn iter(&self) -> ElementIter<'a, I> {
        ElementIter::new(self.space, self.id.clone())
    }
}
impl<'a, I: 'a + Fits64> IntoIterator for &'_ SpaceRef<'a, Set64<I>> {
    type Item = SpaceRef<'a, I>;

    type IntoIter = ElementIter<'a, I>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a> Element<'a> {
    /// Returns the rank of the element.
    pub fn rank(self) -> u8 {
        self.space.polytopes.lock()[self.id].rank()
    }
    /// Returns an iterator over the boundary of the element. Each element of
    /// the boundary has rank one less than the original element.
    pub fn boundary(self) -> ElementIter<'a> {
        ElementIter::new(
            self.space,
            self.space.polytopes.lock()[self.id]
                .boundary()
                .cloned()
                .unwrap_or_default(),
        )
    }

    /// Returns the element if it is a polytope; otherwise returns an error.
    pub fn as_polytope(self) -> Result<Polytope<'a>> {
        self.as_rank(self.space.ndim(), PolytopeId)
    }
    /// Returns the element if it is a facet; otherwise returns an error.
    pub fn as_facet(self) -> Result<Facet<'a>> {
        self.as_rank(self.space.ndim() - 1, FacetId)
    }
    /// Returns the element if it is a face; otherwise returns an error.
    pub fn as_face(self) -> Result<Face<'a>> {
        self.as_rank(2, FaceId)
    }
    /// Returns the element if it is an edge; otherwise returns an error.
    pub fn as_edge(self) -> Result<Edge<'a>> {
        self.as_rank(1, EdgeId)
    }
    /// Returns the element if it is a vertex; otherwise returns an error.
    pub fn as_vertex(self) -> Result<Vertex<'a>> {
        let id = self.space.polytopes.lock()[self.id]
            .to_vertex()
            .ok_or_eyre("expected vertex; got higher element")?;
        Ok(Vertex {
            space: self.space,
            id,
        })
    }

    /// Returns the element if it has rank `rank`; otherwise returns an error.
    fn as_rank<I>(self, rank: u8, wrap_id: fn(u32) -> I) -> Result<SpaceRef<'a, I>> {
        if self.rank() != rank {
            bail!(
                "expected element with rank {rank}; got rank {}",
                self.rank()
            );
        }
        Ok(SpaceRef::new(self.space, wrap_id(self.id.0)))
    }
    fn from_rank<I>(elem: SpaceRef<'a, I>, unwrap_id: fn(I) -> u32) -> Self {
        Self::new(elem.space, ElementId(unwrap_id(elem.id)))
    }
}

impl<'a> From<Polytope<'a>> for Element<'a> {
    fn from(value: Polytope<'a>) -> Self {
        Element::from_rank(value, |i| i.0)
    }
}
impl<'a> Polytope<'a> {
    /// Returns the facets of the polytope.
    pub fn facets(self) -> MappedElementIter<'a, FacetId> {
        self.as_element()
            .boundary()
            .map(|e| e.map_id(|e| FacetId(e.0)))
    }
    /// Returns whether any facet of the polytope is primordial (i.e., whether
    /// this polytope touches the boundary of the primordial cube).
    pub fn has_primordial_facet(self) -> bool {
        self.facets().any(|f| f.is_primordial())
    }
}

impl<'a> From<Facet<'a>> for Element<'a> {
    fn from(value: Facet<'a>) -> Self {
        Element::from_rank(value, |i| i.0)
    }
}
impl<'a> Facet<'a> {
    /// Returns the ridges of the facet.
    pub fn ridges(self) -> ElementIter<'a> {
        self.as_element().boundary()
    }
    /// Returns whether the facet is primordial (i.e., whether it is on the
    /// boundary of the primordial cube).
    pub fn is_primordial(self) -> bool {
        match self.space.polytopes.lock()[self.as_element().id] {
            PolytopeData::Polytope { is_primordial, .. } => is_primordial,
            _ => false,
        }
    }
    /// Returns the hyperplane of the facet.
    pub fn hyperplane(self) -> Result<Hyperplane> {
        self.space.hyperplane_of_facet(self.id)
    }
}

impl<'a> From<Face<'a>> for Element<'a> {
    fn from(value: Face<'a>) -> Self {
        Element::from_rank(value, |i| i.0)
    }
}
impl<'a> Face<'a> {
    /// Returns the edges of the face, in any order.
    pub fn edges(self) -> MappedElementIter<'a, EdgeId> {
        self.as_element()
            .boundary()
            .map(|e| e.map_id(|e| EdgeId(e.0)))
    }
    /// Returns the endpoints of each edge in the face, in cyclic order.
    pub fn edge_endpoints(self) -> Result<impl Iterator<Item = [Vertex<'a>; 2]>> {
        let mut adjacency_map = HashMap::<VertexId, SmallVec<[VertexId; 2]>>::new();

        for edge in self.edges() {
            let [a, b] = edge.endpoints()?;
            adjacency_map.entry(a.id).or_default().push(b.id);
            adjacency_map.entry(b.id).or_default().push(a.id);
        }

        let init = self.arbitrary_vertex()?;
        let space = self.space;
        let mut take_adjacent = move |v: Vertex<'_>| {
            let adj = adjacency_map.get_mut(&v.id)?.pop()?;
            adjacency_map.get_mut(&adj)?.retain(|u| *u != v.id);
            Some(space.get(adj))
        };

        let init_edge = [init, take_adjacent(init).ok_or_eyre("bad polygon")?];
        Ok(std::iter::successors(Some(init_edge), move |&[_, b]| {
            Some([b, take_adjacent(b)?])
        }))
    }
    /// Returns an orthonormal pair of vectors spanning the face.
    pub fn tangent_vectors(self) -> Result<[Vector; 2]> {
        // IIFE to mimic try_block
        (|| {
            // This algorithm runs in O(n) time rather than O(1) because we want to
            // select a "good" triplet of vertices to maintain numerical precision.
            let mut verts = self.vertex_set().map(|v| v.pos()).collect_vec();
            // Pick one vertex arbitrarily to start.
            let initial_vertex = verts.pop()?;

            // Compute the delta from the initial vertex to each remaining point.
            let mut tangent_vectors = verts.into_iter().map(|v| v - &initial_vertex).collect_vec();

            // Pick the longest tangent vector, then normalize it.
            let i = tangent_vectors
                .iter()
                .position_max_by_key(|v| FloatOrd(v.mag2()))?;
            let u_tangent = tangent_vectors.swap_remove(i).normalize()?;

            // Orthogonalize the remaining vectors.
            let mut tangent_vectors = tangent_vectors
                .into_iter()
                .filter_map(|v| v.rejected_from(&u_tangent))
                .collect_vec();

            // Pick the longest tangent vector, then normalize it.
            let i = tangent_vectors
                .iter()
                .position_max_by_key(|v| FloatOrd(v.mag2()))?;
            let v_tangent = tangent_vectors.swap_remove(i).normalize()?;

            Some([u_tangent, v_tangent])
        })()
        .ok_or_eyre("degenerate face")
    }
}

impl<'a> From<Edge<'a>> for Element<'a> {
    fn from(value: Edge<'a>) -> Self {
        Element::from_rank(value, |i| i.0)
    }
}
impl<'a> Edge<'a> {
    /// Returns the endpoints of the edge.
    pub fn endpoints(self) -> Result<[Vertex<'a>; 2]> {
        let space = self.space;
        let [a, b] = <[_; 2]>::try_from(self.as_element().boundary().collect_vec())
            .map_err(|e| eyre!("bad edge boundary set: {e:?}"))?;
        let polytopes = space.polytopes.lock();
        Ok([
            polytopes[a.id].to_vertex().ok_or_eyre("bad edge")?,
            polytopes[b.id].to_vertex().ok_or_eyre("bad edge")?,
        ]
        .map(|id| Vertex { space, id }))
    }
}

impl<'a> From<Vertex<'a>> for Element<'a> {
    fn from(value: Vertex<'a>) -> Self {
        Element::new(value.space, value.space.vertex_to_polytope(value.id))
    }
}
impl Vertex<'_> {
    /// Returns the position of the vertex.
    pub fn pos(self) -> Vector {
        self.space.vertices.lock()[self.id].clone()
    }
}

/// Iterator over a set of elements of a particular type.
pub type MappedElementIter<'a, U> =
    std::iter::Map<ElementIter<'a>, fn(Element<'a>) -> SpaceRef<'a, U>>;

/// Iterator over a set of elements.
pub struct ElementIter<'a, T: Fits64 = ElementId> {
    space: &'a Space,
    ids: tinyset::set64::IntoIter<T>,
}
impl<'a, T: Fits64> Iterator for ElementIter<'a, T> {
    type Item = SpaceRef<'a, T>;

    fn next(&mut self) -> Option<Self::Item> {
        self.ids.next().map(|id| SpaceRef {
            space: self.space,
            id,
        })
    }
}
impl<'a, T: Fits64> ElementIter<'a, T> {
    fn new(space: &'a Space, ids: Set64<T>) -> Self {
        Self {
            space,
            ids: ids.into_iter(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_face_edge_endpoints() -> Result<()> {
        let space = Space::new(2);
        let square = space.add_primordial_cube(3.0)?.as_element().as_face()?;
        let edge_endpoints = square.edge_endpoints()?.collect_vec();
        println!("{edge_endpoints:?}");
        assert_eq!(edge_endpoints.len(), 4);
        Ok(())
    }
}
