use std::ops::Range;

use eyre::{OptionExt, Result, bail, ensure};
use hypermath::prelude::*;

use super::{PerGizmoFace, PerPiece, PerSticker, Piece, Surface};
use crate::GizmoFace;

/// Data to render a puzzle, in a format that can be sent to the GPU.
#[derive(Debug, Clone)]
pub struct Mesh {
    /// Number of dimensions of the mesh.
    pub ndim: u8,

    /// Number of sticker colors in the mesh.
    pub color_count: usize,
    /// Number of polygons in the mesh.
    pub polygon_count: usize,
    /// Number of stickers in the mesh.
    pub sticker_count: usize,
    /// Number of pieces in the mesh.
    pub piece_count: usize,
    /// Number of puzzle surfaces in the mesh.
    pub puzzle_surface_count: usize,
    /// Number of puzzle vertices in the mesh.
    pub puzzle_vertex_count: usize,

    /// Number of twist gizmo faces in the mesh.
    pub gizmo_face_count: usize,
    /// Number of twist gizmo surfaces in the mesh.
    pub gizmo_surface_count: usize,
    /// Number of twist gizmo vertices in the mesh.
    pub gizmo_vertex_count: usize,

    /// Coordinates for each vertex in N-dimensional space.
    pub vertex_positions: Vec<f32>,
    /// First tangent vector for each vertex, used to compute surface normal.
    pub u_tangents: Vec<f32>,
    /// Second tangent vector for each vertex, used to compute surface normal.
    pub v_tangents: Vec<f32>,
    /// Vector along which to move each vertex when applying sticker shrink.
    pub sticker_shrink_vectors: Vec<f32>,
    /// Piece ID for each vertex.
    pub piece_ids: Vec<u32>,
    /// Surface ID for each vertex.
    pub surface_ids: Vec<u32>,
    /// Polygon ID for each vertex. Each polygon is a single color.
    pub polygon_ids: Vec<u32>,

    /// Centroid for each piece, used to apply piece explode.
    pub piece_centroids: Vec<f32>,
    /// Centroid for each surface, used to apply facet shrink.
    pub surface_centroids: Vec<f32>,
    /// Normal vector for each surface, used to cull 4D backfaces.
    pub surface_normals: Vec<f32>,

    /// For each sticker, the range of polygon IDs it spans.
    pub sticker_polygon_ranges: PerSticker<Range<usize>>,
    /// For each piece, the range of polygon IDs its internals spans.
    pub piece_internals_polygon_ranges: PerPiece<Range<usize>>,

    /// Vertex indices for triangles.
    pub triangles: Vec<[u32; 3]>,
    /// For each sticker, the range in `triangles` containing its triangles.
    pub sticker_triangle_ranges: PerSticker<Range<u32>>,
    /// For each piece, the range in `triangles` containing its internals'
    /// triangles.
    pub piece_internals_triangle_ranges: PerPiece<Range<u32>>,
    /// For each twist gizmo, the range in `triangles` contains its triangles.
    pub gizmo_triangle_ranges: PerGizmoFace<Range<u32>>,

    /// Vertex indices for edges.
    pub edges: Vec<[u32; 2]>,
    /// For each sticker, the range in `edges` containing its edges.
    pub sticker_edge_ranges: PerSticker<Range<u32>>,
    /// For each piece, the range in `edges` containing its internals' edges.
    pub piece_internals_edge_ranges: PerPiece<Range<u32>>,
    /// For each twist gizmo, the range in `edges` containing its edges.
    pub gizmo_edge_ranges: PerGizmoFace<Range<u32>>,
}

impl Default for Mesh {
    fn default() -> Self {
        Self::new_empty(1)
    }
}

impl Mesh {
    /// Constructs an empty mesh.
    pub const fn new_empty(ndim: u8) -> Self {
        Mesh {
            ndim,
            color_count: 0,
            polygon_count: 0,
            sticker_count: 0,
            piece_count: 0,
            puzzle_surface_count: 0,
            puzzle_vertex_count: 0,

            gizmo_face_count: 0,
            gizmo_surface_count: 0,
            gizmo_vertex_count: 0,

            vertex_positions: vec![],
            u_tangents: vec![],
            v_tangents: vec![],
            sticker_shrink_vectors: vec![],
            piece_ids: vec![],
            surface_ids: vec![],
            polygon_ids: vec![],

            piece_centroids: vec![],
            surface_centroids: vec![],
            surface_normals: vec![],

            triangles: vec![],
            sticker_triangle_ranges: PerSticker::new(),
            piece_internals_triangle_ranges: PerPiece::new(),
            gizmo_triangle_ranges: PerGizmoFace::new(),

            edges: vec![],
            sticker_edge_ranges: PerSticker::new(),
            piece_internals_edge_ranges: PerPiece::new(),

            sticker_polygon_ranges: PerSticker::new(),
            piece_internals_polygon_ranges: PerPiece::new(),
            gizmo_edge_ranges: PerGizmoFace::new(),
        }
    }

    /// Returns whether the puzzle has no vertices.
    pub fn is_empty(&self) -> bool {
        self.vertex_count() == 0
    }

    /// Returns the number of surfaces in the mesh, including puzzle surfaces
    /// and twist gizmo surfaces.
    pub fn surface_count(&self) -> usize {
        self.puzzle_surface_count + self.gizmo_surface_count
    }
    /// Returns the number of vertices in the mesh, including puzzle vertices
    /// and twist gizmo vertices.
    pub fn vertex_count(&self) -> usize {
        self.puzzle_vertex_count + self.gizmo_vertex_count
    }
    /// Returns the number of facets in the mesh.
    pub fn facet_count(&self) -> usize {
        self.surface_centroids.len() / self.ndim as usize
    }
    /// Returns the number of triangles in the mesh.
    pub fn triangle_count(&self) -> usize {
        self.triangles.len()
    }
    /// Returns the number of edges in the mesh.
    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }

    /// Adds a vertex to the mesh and returns the vertex ID.
    pub fn add_puzzle_vertex(&mut self, data: MeshVertexData<'_>) -> Result<u32> {
        ensure!(
            self.gizmo_vertex_count == 0,
            "puzzle mesh must be constructed before twist gizmos",
        );

        let ndim = self.ndim;
        let vertex_id = self.vertex_count() as u32;
        self.puzzle_vertex_count += 1;
        self.vertex_positions
            .extend(iter_f32(ndim, data.position.as_vector()));
        self.u_tangents.extend(iter_f32(ndim, data.u_tangent));
        self.v_tangents.extend(iter_f32(ndim, data.v_tangent));
        self.sticker_shrink_vectors
            .extend(iter_f32(ndim, data.sticker_shrink_vector));
        self.piece_ids.push(data.piece_id.0);
        self.surface_ids.push(data.surface_id.0 as u32);
        self.polygon_ids.push(data.polygon_id);

        Ok(vertex_id)
    }
    /// Adds a gizmo vertex to the mesh and returns the vertex ID.
    pub fn add_gizmo_vertex(&mut self, pos: Point, surface_id: u32) -> Result<u32> {
        let ndim = self.ndim;
        let vertex_id = self.vertex_count() as u32;
        self.gizmo_vertex_count += 1;
        self.vertex_positions
            .extend(iter_f32(ndim, pos.as_vector()));
        // No tangent vectors needed.
        // No sticker shrink vectors needed.
        // No piece ID needed.
        // We *do* need a surface ID.
        self.surface_ids.push(surface_id);
        // No polygon ID needed.

        Ok(vertex_id)
    }
    /// Adds a polygon to the mesh and returns its ID.
    pub fn next_polygon_id(&mut self) -> Result<u32> {
        let ret = self.polygon_count as u32;
        self.polygon_count = self
            .polygon_count
            .checked_add(1)
            .ok_or_eyre("too many polygons")?;
        Ok(ret)
    }

    /// Adds a sticker to the mesh.
    pub fn add_sticker(
        &mut self,
        polygon_range: Range<usize>,
        triangle_range: Range<u32>,
        edge_range: Range<u32>,
    ) -> Result<()> {
        self.sticker_count += 1;
        self.sticker_polygon_ranges.push(polygon_range)?;
        self.sticker_triangle_ranges.push(triangle_range)?;
        self.sticker_edge_ranges.push(edge_range)?;

        Ok(())
    }
    /// Adds a piece to the mesh.
    pub fn add_piece(
        &mut self,
        centroid: &Point,
        internals_polygon_range: Range<usize>,
        internals_triangle_range: Range<u32>,
        internals_edge_range: Range<u32>,
    ) -> Result<()> {
        let ndim = self.ndim;
        self.piece_count += 1;
        self.piece_centroids
            .extend(iter_f32(ndim, centroid.as_vector()));
        self.piece_internals_polygon_ranges
            .push(internals_polygon_range)?;
        self.piece_internals_triangle_ranges
            .push(internals_triangle_range)?;
        self.piece_internals_edge_ranges
            .push(internals_edge_range)?;

        Ok(())
    }

    /// Adds a gizmo face to the mesh.
    pub fn add_gizmo_face(
        &mut self,
        triangle_range: Range<u32>,
        edge_range: Range<u32>,
    ) -> Result<GizmoFace> {
        let ret = GizmoFace(self.gizmo_face_count as _);
        self.gizmo_face_count += 1;
        self.gizmo_triangle_ranges.push(triangle_range)?;
        self.gizmo_edge_ranges.push(edge_range)?;
        Ok(ret)
    }

    /// Adds a puzzle surface to the mesh and returns the new surface ID.
    ///
    /// This cannot be called after `add_gizmo_surface()`.
    pub fn add_puzzle_surface(&mut self, centroid: &Point, normal: impl VectorRef) -> Result<u32> {
        let surface_id = self.surface_count() as u32;
        let ndim = self.ndim;
        self.puzzle_surface_count += 1;
        if self.gizmo_surface_count > 0 {
            bail!("puzzle surfaces must precede gizmo surfaces");
        }
        self.surface_centroids
            .extend(iter_f32(ndim, centroid.as_vector()));
        self.surface_normals.extend(iter_f32(ndim, &normal));
        Ok(surface_id)
    }

    /// Adds a gizmo surface to the mesh and returns the new surface ID.
    pub fn add_gizmo_surface(&mut self, axis_vector: impl VectorRef) -> Result<u32> {
        let normal = axis_vector
            .normalize()
            .ok_or_eyre("axis vector cannot be zero")?;
        let centroid = axis_vector;

        let ndim = self.ndim;
        let surface_id = self.surface_count() as u32;
        self.gizmo_surface_count += 1;
        self.surface_centroids.extend(iter_f32(ndim, &centroid));
        self.surface_normals.extend(iter_f32(ndim, &normal));

        Ok(surface_id)
    }

    /// Returns the position of the `i`th vertex.
    pub fn vertex_position(&self, i: u32) -> Vector {
        self.index_vertex_vector(&self.vertex_positions, i)
    }
    /// Returns the U tangent of the `i`th vertex.
    pub fn u_tangent(&self, i: u32) -> Vector {
        self.index_vertex_vector(&self.u_tangents, i)
    }
    /// Returns the V tangent of the `i`th vertex.
    pub fn v_tangent(&self, i: u32) -> Vector {
        self.index_vertex_vector(&self.v_tangents, i)
    }
    fn index_vertex_vector(&self, v: &[f32], i: u32) -> Vector {
        let start = i as usize * self.ndim as usize;
        let end = (i + 1) as usize * self.ndim as usize;
        v[start..end].iter().map(|&x| x as _).collect()
    }
}

/// Vertex that can be added to a mesh.
#[derive(Debug, Copy, Clone)]
pub struct MeshVertexData<'a> {
    /// N-dimensional coordinates of the point.
    pub position: &'a Point,
    /// N-dimensional unit vector tangent to the surface at the point. This must
    /// be perpendicular to `v_tangent`.
    pub u_tangent: &'a Vector,
    /// N-dimensional unit vector tangent to the surface at the point. This must
    /// be perpendicular to `u_tangent`.
    pub v_tangent: &'a Vector,
    /// Vector along which to shrink the vertex for sticker shrink.
    pub sticker_shrink_vector: &'a Vector,
    /// ID of the piece that the vertex is part of. This is used for piece
    /// explode.
    pub piece_id: Piece,
    /// ID of the surface that the vertex is part of. This is used for facet
    /// shrink.
    pub surface_id: Surface,
    /// ID of the polygon that the vertex is part of. This is used for
    /// determining color and lighting.
    pub polygon_id: u32,
}

/// Returns an iterator over the components of `v` as `f32`s, padded to `ndim`.
fn iter_f32(ndim: u8, v: &impl VectorRef) -> impl '_ + Iterator<Item = f32> {
    v.iter_ndim(ndim).map(|x| x as f32)
}
