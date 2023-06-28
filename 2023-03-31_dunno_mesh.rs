/// Vertex data for a mesh.
///
/// The `Vec<f32>` in this struct are all flattened arrays of N-dimensional
/// vectors.
pub struct MeshVertexData {
    ndim: u8,

    /// Vertex coordinates in N-dimensional space.
    pub points: Vec<f32>,

    /// First tangent vector, used to compute surface normal.
    pub u_tangents: Vec<f32>,
    /// Second tangent vector, used to compute surface normal.
    pub v_tangents: Vec<f32>,

    /// Vector along which to move a vertex when applying sticker shrink.
    ///
    /// TODO: this should maybe be a conformal transformation that shrinks
    /// within the facet manifold.
    pub sticker_shrink_vectors: Vec<f32>,

    /// Facet ID (or `-1` if internal), used to apply facet shrink and to color
    /// the vertex.
    pub facet_ids: Vec<FacetId>,
    /// Piece ID, used to apply piece explode.
    pub piece_ids: Vec<PieceId>,

    /// Vertex indices.
    pub indices: Vec<u32>,
}
impl MeshVertexData {
    fn push_vertex(&mut self, vertex: Vertex) {
        self.points.extend(vertex.point.iter_ndim(self.ndim));

        self.u_tangents
            .extend(vertex.u_tangent.iter_ndim(self.ndim));
        self.v_tangents
            .extend(vertex.v_tangent.iter_ndim(self.ndim));

        self.sticker_shrink_vectors
            .extend(vertex.sticker_shrink_vector.iter_ndim(self.ndim));

        self.facet_ids.push(vertex.facet_id);
        self.piece_ids.push(vertex.piece_id);
    }
    fn push_tri(&mut self, indices: [u32; 3]) {
        self.indices.extend(indices)
    }

    /// Returns the number of dimensions of the points in the mesh.
    pub fn ndim(&self) -> u8 {
        self.ndim
    }
    /// Returns the number of vertices in the mesh.
    pub fn num_verts(&self) -> usize {
        self.points.len() / self.ndim as usize
    }
    /// Returns the number of triangles in the mesh.
    pub fn num_tris(&self) -> usize {
        self.indices.len() / 3
    }
}
struct Vertex {
    point: Vector,

    u_tangent: Vector,
    v_tangent: Vector,

    sticker_shrink_vector: Vector,
    facet_id: FacetId,
    piece_id: PieceId,
}
