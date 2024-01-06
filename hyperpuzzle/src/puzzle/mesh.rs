use std::ops::Range;

use eyre::{OptionExt, Result};
use hypermath::prelude::*;

use super::{Color, PerPiece, PerSticker, Piece};
use crate::Facet;

/// Data to render a puzzle, in a format that can be sent to the GPU.
#[derive(Debug, Clone)]
pub struct Mesh {
    ndim: u8,

    /// Number of sticker colors in the mesh.
    pub color_count: usize,
    /// Number of polygons in the mesh.
    pub polygon_count: usize,

    /// Coordinates for each vertex in N-dimensional space.
    pub vertex_positions: Vec<f32>,
    /// First tangent vector for each vertex, used to compute surface normal.
    pub u_tangents: Vec<f32>,
    /// Second tangent vector for each vertex, used to compute surface normal.
    pub v_tangents: Vec<f32>,
    /// Vector along which to move each vertex when applying sticker shrink.
    pub sticker_shrink_vectors: Vec<f32>,
    /// Piece ID for each vertex.
    pub piece_ids: Vec<Piece>,
    /// Facet ID for each vertex.
    pub facet_ids: Vec<Facet>,
    /// Polygon ID for each vertex. Outlines are drawn between polygons.
    pub polygon_ids: Vec<u32>,

    /// Color ID for each polygon.
    pub polygon_color_ids: Vec<Color>,

    /// Centroid for each piece, used to apply piece explode.
    pub piece_centroids: Vec<f32>,
    /// Centroid for each facet, used to apply facet shrink.
    pub facet_centroids: Vec<f32>,
    /// Normal vector for each facet, used to cull 4D backfaces.
    pub facet_normals: Vec<f32>,

    /// Vertex indices for triangles.
    pub triangles: Vec<[u32; 3]>,

    /// For each sticker, the range in `triangles` containing its triangles.
    pub sticker_index_ranges: PerSticker<Range<u32>>,
    /// For each piece, the range in `triangles` containing its internals'
    /// triangles.
    pub piece_internals_index_ranges: PerPiece<Range<u32>>,
}

impl Default for Mesh {
    fn default() -> Self {
        Self::new_empty(1)
    }
}

impl Mesh {
    /// Constructs an empty mesh.
    pub fn new_empty(ndim: u8) -> Self {
        Mesh {
            ndim,
            color_count: 0,
            polygon_count: 0,

            vertex_positions: vec![],
            u_tangents: vec![],
            v_tangents: vec![],
            sticker_shrink_vectors: vec![],
            piece_ids: vec![],
            facet_ids: vec![],
            polygon_ids: vec![],

            polygon_color_ids: vec![],

            piece_centroids: vec![],
            facet_centroids: vec![],
            facet_normals: vec![],

            triangles: vec![],

            sticker_index_ranges: PerSticker::new(),
            piece_internals_index_ranges: PerPiece::new(),
        }
    }

    /// Returns the number of dimensions of the mesh.
    pub fn ndim(&self) -> u8 {
        self.ndim
    }
    /// Returns the number of vertices in the mesh.
    pub fn vertex_count(&self) -> usize {
        self.vertex_positions.len() / self.ndim as usize
    }
    /// Returns the number of pieces in the mesh.
    pub fn piece_count(&self) -> usize {
        self.piece_internals_index_ranges.len()
    }
    /// Returns the number of facets in the mesh.
    pub fn facet_count(&self) -> usize {
        self.facet_centroids.len() / self.ndim as usize
    }
    /// Returns the number of stickers in the mesh.
    pub fn sticker_count(&self) -> usize {
        self.sticker_index_ranges.len()
    }
    /// Returns the number of triangles in the mesh.
    pub fn triangle_count(&self) -> usize {
        self.triangles.len()
    }
    /// Returns the number of colors in the mesh.
    pub fn color_count(&self) -> usize {
        self.color_count
    }

    pub(super) fn add_vertex(&mut self, data: MeshVertexData<'_>) -> u32 {
        let vertex_id = self.vertex_count() as u32;

        let ndim = self.ndim();
        self.vertex_positions.extend(iter_f32(ndim, data.position));
        self.u_tangents.extend(iter_f32(ndim, data.u_tangent));
        self.v_tangents.extend(iter_f32(ndim, data.v_tangent));
        self.sticker_shrink_vectors
            .extend(iter_f32(ndim, data.sticker_shrink_vector));
        self.piece_ids.push(data.piece_id);
        self.facet_ids.push(data.facet_id);
        self.polygon_ids.push(data.polygon_id);

        vertex_id
    }
    pub(super) fn next_polygon_id(&mut self) -> Result<u32> {
        let ret = self.polygon_count as u32;
        self.polygon_count = self
            .polygon_count
            .checked_add(1)
            .ok_or_eyre("too many polygons")?;
        Ok(ret)
    }

    /// Adds a piece to the mesh, given its centroid and a function to add its
    /// internal facets.
    pub(super) fn add_piece(
        &mut self,
        centroid: &impl VectorRef,
        internals_index_range: Range<u32>,
    ) -> Result<()> {
        let ndim = self.ndim();
        self.piece_centroids.extend(iter_f32(ndim, centroid));
        self.piece_internals_index_ranges
            .push(internals_index_range)?;

        Ok(())
    }

    pub(super) fn add_facet(
        &mut self,
        centroid: impl VectorRef,
        normal: impl VectorRef,
    ) -> Result<()> {
        let ndim = self.ndim();
        self.facet_centroids.extend(iter_f32(ndim, &centroid));
        self.facet_normals.extend(iter_f32(ndim, &normal));

        Ok(())
    }
}

#[derive(Debug, Copy, Clone)]
pub(super) struct MeshVertexData<'a> {
    pub position: &'a Vector,
    pub u_tangent: &'a Vector,
    pub v_tangent: &'a Vector,
    pub sticker_shrink_vector: &'a Vector,
    pub piece_id: Piece,
    pub facet_id: Facet,
    pub polygon_id: u32,
}

fn iter_f32(ndim: u8, v: &impl VectorRef) -> impl '_ + Iterator<Item = f32> {
    v.iter_ndim(ndim).map(|x| x as f32)
}
