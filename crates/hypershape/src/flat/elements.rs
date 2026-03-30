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
impl ToElementId for VertexId {
    fn to_element_id(&self, space: &Space) -> ElementId {
        space.vertex_to_polytope(*self)
    }
}

macro_rules! impl_trivial_to_element_id {
    ($ty:ty) => {
        impl ToElementId for $ty {
            fn to_element_id(&self, _space: &Space) -> ElementId {
                ElementId(self.0)
            }
        }
        impl From<$ty> for ElementId {
            fn from(value: $ty) -> Self {
                ElementId(value.0)
            }
        }
    };
}

impl_trivial_to_element_id!(PolytopeId);
impl_trivial_to_element_id!(FacetId);
impl_trivial_to_element_id!(FaceId);
impl_trivial_to_element_id!(EdgeId);

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
        self.space.polytopes[self.id].rank()
    }
    /// Returns an iterator over the boundary of the element. Each element of
    /// the boundary has rank one less than the original element.
    pub fn boundary(self) -> ElementIter<'a> {
        ElementIter::new(
            self.space,
            self.space.polytopes[self.id]
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
        let id = self.space.polytopes[self.id]
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

    /// Computes the sticker shrink vectors for a polytope.
    ///
    /// Each vertex shrinks along a vector pointing toward the centroid of the
    /// polytope, projected onto whatever sticker facets that vertex is part of.
    /// For example, if a vertex is on an edge (1D manifold) of a 3D polytope,
    /// then its shrink vector will point toward the centroid of the polytope,
    /// projected onto that edge. If a vertex is on a corner of its polytope,
    /// then its shrink vector is zero.
    ///
    /// `sticker_facets` must _not_ include internal facets.
    ///
    /// This is a relatively expensive operation, so it should be called only
    /// once per polytope.
    pub fn sticker_shrink_vectors(
        &self,
        sticker_facets: &[FacetId],
    ) -> Result<HashMap<VertexId, Vector>> {
        let space = self.space();

        // For each element of the polytope, compute a set of the sticker facets
        // that contain the element.
        let mut elements_and_facet_sets_by_rank: Vec<HashMap<ElementId, Set64<FacetId>>> =
            vec![HashMap::new(); space.ndim() as usize + 1];
        for &sticker_facet in sticker_facets {
            for ridge in space.get(sticker_facet).subelements() {
                let rank = ridge.rank();
                elements_and_facet_sets_by_rank[rank as usize]
                    .entry(ridge.id())
                    .or_default()
                    .insert(sticker_facet);
            }
        }

        // Find the largest (by rank) elements contained by all the sticker facets
        // of the piece.
        let centroid_of_greatest_common_elements: Option<Centroid> =
            elements_and_facet_sets_by_rank
                .iter()
                .rev()
                .map(|elements_and_facet_sets| {
                    // Find elements that are contained by all sticker facets of the
                    // piece.
                    let elements_with_maximal_facet_set = elements_and_facet_sets
                        .iter()
                        .filter(|(_element, facet_set)| facet_set.len() == sticker_facets.len())
                        .map(|(element, _facet_set)| *element);
                    // Add up their centroids. Technically we should take the centroid
                    // of their convex hull, but this works well enough.
                    space.combined_centroid(elements_with_maximal_facet_set)
                })
                // Select the elements with the largest rank and nonzero centroid.
                .find_map(|result_option| result_option.transpose())
                .transpose()?;
        // If such elements exist, then all vertices can shrink to the same point.
        if let Some(centroid) = centroid_of_greatest_common_elements {
            let shrink_target = centroid.center();
            return Ok(self
                .vertex_set()
                .map(|v| (v.id(), &shrink_target - v.pos()))
                .collect());
        }

        // Otherwise, find the best elements for each set of facets. If a vertex is
        // not contained by any facets, then it will shrink toward the centroid of
        // the piece.
        let piece_centroid = self.centroid()?.center();

        // Compute the shrink target for each possible facet set that has a good
        // shrink target.
        let unique_facet_sets_of_vertices = elements_and_facet_sets_by_rank[0].values().unique();
        let shrink_target_by_surface_set: HashMap<&Set64<FacetId>, Point> =
            unique_facet_sets_of_vertices
                .map(|facet_set| {
                    // Find the largest elements of the piece that are contained by all
                    // the facets in this set. There must be at least one vertex.
                    let centroid_of_greatest_common_elements: Centroid =
                        elements_and_facet_sets_by_rank
                            .iter()
                            .rev()
                            .map(|elements_and_facet_sets| {
                                // Find elements that are contained by all sticker facets of
                                // the vertex.
                                let elements_with_superset_of_facets = elements_and_facet_sets
                                    .iter()
                                    .filter(|(_element, fs)| {
                                        facet_set.iter().all(|f| fs.contains(f))
                                    })
                                    .map(|(element, _fs)| *element);
                                // Add up their centroids. Technically we should take the
                                // centroid of their convex hull, but this works well
                                // enough.
                                space.combined_centroid(elements_with_superset_of_facets)
                            })
                            // Select the elements with the largest rank.
                            .find_map(|result_option| result_option.transpose())
                            // There must be some element with a superset of `facet_set`
                            // because `facet_set` came from a vertex.
                            .ok_or_eyre("no element with facet subset")??;

                    eyre::Ok((facet_set, centroid_of_greatest_common_elements.center()))
                })
                .try_collect()?;

        // Compute shrink vectors for all vertices.
        let empty_set = Set64::new();
        let shrink_vectors = self.vertex_set().map(|vertex| {
            let surface_set = &elements_and_facet_sets_by_rank[0]
                .get(&vertex.as_element().id())
                .unwrap_or(&empty_set);
            let vertex_pos = vertex.pos();
            let shrink_vector = match shrink_target_by_surface_set.get(surface_set) {
                Some(target) => target - vertex_pos,
                None => &piece_centroid - vertex_pos,
            };

            (vertex.id(), shrink_vector)
        });
        Ok(shrink_vectors.collect())
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
        match self.space.polytopes[self.as_element().id] {
            PolytopeData::Polytope { is_primordial, .. } => is_primordial,
            _ => false,
        }
    }
    /// Returns the unoriented hyperplane of the facet.
    pub fn hyperplane(self) -> Result<Hyperplane> {
        match self.space.polytopes[self.as_element().id] {
            PolytopeData::Vertex(vertex_id) if self.space.ndim() == 1 => {
                Hyperplane::new(vector![1.0], self.space.get(vertex_id).pos()[0])
                    .ok_or_eyre("error constructing hyperplane")
            }
            PolytopeData::Polytope {
                hyperplane: Some(hyperplane_id),
                ..
            } => Ok(self.space.hyperplanes.get(hyperplane_id)?.clone()),
            _ => bail!("expected hyperplane"),
        }
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
    /// Returns the vertices of the face, in cyclic order.
    pub fn vertices_in_order(self) -> Result<impl Iterator<Item = Vertex<'a>>> {
        Ok(self.edges_in_order()?.map(|[v1, _v2]| v1))
    }
    /// Returns the endpoints of each edge in the face, in cyclic order.
    pub fn edges_in_order(self) -> Result<impl Iterator<Item = [Vertex<'a>; 2]>> {
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
            .map_err(|e| eyre!("bad edge boundary set: {e:?}"))?
            .map(|v| self.space.polytopes[v.id].to_vertex());
        Ok([a.ok_or_eyre("bad edge")?, b.ok_or_eyre("bad edge")?].map(|id| Vertex { space, id }))
    }
}

impl<'a> From<Vertex<'a>> for Element<'a> {
    fn from(value: Vertex<'a>) -> Self {
        Element::new(value.space, value.space.vertex_to_polytope(value.id))
    }
}
impl Vertex<'_> {
    /// Returns the position of the vertex.
    pub fn pos(self) -> Point {
        self.space.vertex_pos(self.id)
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
        let space = Space::with_primordial_cube_radius(2, 3.0)?;
        let square = space.get(space.primordial_cube()).as_element().as_face()?;
        let edge_endpoints = square.edges_in_order()?.collect_vec();
        println!("{edge_endpoints:?}");
        assert_eq!(edge_endpoints.len(), 4);
        Ok(())
    }
}
