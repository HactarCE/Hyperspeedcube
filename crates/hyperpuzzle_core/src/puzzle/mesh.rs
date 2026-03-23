use std::ops::Range;

use eyre::{OptionExt, Result, bail, ensure};
use hypermath::prelude::*;
use itertools::Itertools;

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

    /// Vertex indices for triangles.
    pub triangles: Vec<[u32; 3]>,
    /// Vertex indices for edges.
    pub edges: Vec<[u32; 2]>,

    /// For each sticker, the portion of the mesh corresponding to it.
    pub sticker_ranges: PerSticker<MeshRange>,
    /// For each piece, the portion of the mesh corresponding to its internals.
    pub piece_internals_ranges: PerPiece<MeshRange>,
    /// For each twist gizmo, the portion of the mesh corresponding to it.
    pub gizmo_ranges: PerGizmoFace<MeshRange>,
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
            edges: vec![],

            sticker_ranges: PerSticker::new(),
            piece_internals_ranges: PerPiece::new(),
            gizmo_ranges: PerGizmoFace::new(),
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

    /// Returns the number of polygons, triangles, and edges in the mesh.
    pub fn counts(&self) -> MeshCounts {
        MeshCounts {
            vertex_count: self.vertex_count() as u32,
            edge_count: self.edge_count() as u32,
            triangle_count: self.triangle_count() as u32,
            polygon_count: self.polygon_count as u32,
        }
    }

    /// Adds a puzzle polygon to the mesh and returns its range.
    ///
    /// Each vertex consists of a position ([`Point`]) and a sticker shrink
    /// vector ([`Vector`]).
    pub fn add_puzzle_polygon<'a>(
        &mut self,
        vertex_positions: impl IntoIterator<Item = (&'a Point, &'a Vector)>,
        piece_id: Piece,
        surface_id: Surface,
        u_tangent: &Vector,
        v_tangent: &Vector,
    ) -> Result<MeshRange> {
        let start = self.counts();

        let polygon_id = self.next_polygon_id()?;

        // Vertices
        let vertex_start = self.vertex_count() as u32;
        for (position, sticker_shrink_vector) in vertex_positions {
            self.add_puzzle_vertex(MeshVertexData {
                position,
                u_tangent,
                v_tangent,
                sticker_shrink_vector,
                piece_id,
                surface_id,
                polygon_id,
            })?;
        }
        let vertex_end = self.vertex_count() as u32;

        // Edges
        for (v1, v2) in (vertex_start..vertex_end).circular_tuple_windows() {
            self.edges.push([v1, v2]);
        }

        // Triangles
        let v1 = vertex_start;
        for (v2, v3) in ((vertex_start + 1)..vertex_end).tuple_windows() {
            self.triangles.push([v1, v2, v3]);
        }

        let end = self.counts();
        Ok(MeshRange::new(start, end))
    }

    /// Adds a gizmo polygon to the mesh and returns its range.
    pub fn add_gizmo_polygon<'a>(
        &mut self,
        vertex_positions: impl IntoIterator<Item = &'a Point>,
        surface_id: u32,
    ) -> Result<MeshRange> {
        let start = self.counts();

        // Vertices
        let vertex_start = self.vertex_count() as u32;
        for position in vertex_positions {
            self.add_gizmo_vertex(position, surface_id)?;
        }
        let vertex_end = self.vertex_count() as u32;

        // Edges
        for (v1, v2) in (vertex_start..vertex_end).circular_tuple_windows() {
            self.edges.push([v1, v2]);
        }

        // Triangles
        let v1 = vertex_start;
        for (v2, v3) in ((vertex_start + 1)..vertex_end).tuple_windows() {
            self.triangles.push([v1, v2, v3]);
        }

        let end = self.counts();
        Ok(MeshRange::new(start, end))
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
    pub fn add_gizmo_vertex(&mut self, pos: &Point, surface_id: u32) -> Result<u32> {
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
    pub fn add_sticker(&mut self, range: impl Into<MeshRange>) -> Result<()> {
        self.sticker_count += 1;
        self.sticker_ranges.push(range.into())?;

        Ok(())
    }
    /// Adds a piece to the mesh.
    pub fn add_piece(
        &mut self,
        centroid: &Point,
        internals_range: impl Into<MeshRange>,
    ) -> Result<()> {
        let ndim = self.ndim;
        self.piece_count += 1;
        self.piece_centroids
            .extend(iter_f32(ndim, centroid.as_vector()));
        self.piece_internals_ranges.push(internals_range.into())?;

        Ok(())
    }

    /// Adds a gizmo face to the mesh.
    pub fn add_gizmo_face(&mut self, range: impl Into<MeshRange>) -> Result<GizmoFace> {
        let ret = GizmoFace(self.gizmo_face_count as _);
        self.gizmo_face_count += 1;
        self.gizmo_ranges.push(range.into())?;
        Ok(ret)
    }

    /// Adds a puzzle surface to the mesh and returns the new surface ID.
    ///
    /// This cannot be called after `add_gizmo_surface()`.
    pub fn add_puzzle_surface(
        &mut self,
        centroid: &Point,
        normal: impl VectorRef,
    ) -> Result<Surface> {
        let surface_id = self.surface_count() as u16;
        let ndim = self.ndim;
        self.puzzle_surface_count += 1;
        if self.gizmo_surface_count > 0 {
            bail!("puzzle surfaces must precede gizmo surfaces");
        }
        self.surface_centroids
            .extend(iter_f32(ndim, centroid.as_vector()));
        self.surface_normals.extend(iter_f32(ndim, &normal));
        Ok(Surface(surface_id))
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
    ///
    /// This is ignored when rendering internals.
    pub surface_id: Surface,
    /// ID of the polygon that the vertex is part of. This is used for
    /// determining color and lighting.
    pub polygon_id: u32,
}

/// Returns an iterator over the components of `v` as `f32`s, padded to `ndim`.
fn iter_f32(ndim: u8, v: &impl VectorRef) -> impl '_ + Iterator<Item = f32> {
    v.iter_ndim(ndim).map(|x| x as f32)
}

/// Numbers of certain elements in the mesh.
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, Hash)]
pub struct MeshCounts {
    /// Number of vertices in the mesh.
    pub vertex_count: u32,
    /// Number of edges in the mesh.
    pub edge_count: u32,
    /// Number of triangles in the mesh.
    pub triangle_count: u32,
    /// Number of polygons in the mesh.
    pub polygon_count: u32,
}

/// Contiguous portion of a mesh.
///
/// This is basically `std::ops::Range<MeshCounts>` but `Copy`.
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, Hash)]
pub struct MeshRange {
    /// Lower bounds of the range.
    pub start: MeshCounts,
    /// Upper bounds of the range.
    pub end: MeshCounts,
}

impl From<Range<MeshCounts>> for MeshRange {
    fn from(value: Range<MeshCounts>) -> Self {
        Self {
            start: value.start,
            end: value.end,
        }
    }
}

impl MeshRange {
    /// Empty range in a mesh.
    pub const EMPTY: Self = {
        let zero = MeshCounts {
            vertex_count: 0,
            edge_count: 0,
            triangle_count: 0,
            polygon_count: 0,
        };
        Self::new(zero, zero)
    };

    /// Constructs a range from `start` to `end`.
    pub const fn new(start: MeshCounts, end: MeshCounts) -> Self {
        Self { start, end }
    }

    /// Returns the range of vertices from the mesh.
    pub const fn vertex_range(self) -> Range<usize> {
        self.start.vertex_count as usize..self.end.vertex_count as usize
    }
    /// Returns the range of edges from the mesh.
    pub const fn edge_range(self) -> Range<usize> {
        self.start.edge_count as usize..self.end.edge_count as usize
    }
    /// Returns the range of triangles from the mesh.
    pub const fn triangle_range(self) -> Range<usize> {
        self.start.triangle_count as usize..self.end.triangle_count as usize
    }
    /// Returns the range of polygons from the mesh.
    pub const fn polygon_range(self) -> Range<usize> {
        self.start.polygon_count as usize..self.end.polygon_count as usize
    }
}
