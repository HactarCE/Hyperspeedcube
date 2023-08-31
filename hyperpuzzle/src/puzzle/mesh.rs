use std::collections::{hash_map, HashMap};
use std::ops::Range;

use anyhow::{ensure, Context, Result};
use hypermath::prelude::*;
use hypershape::ManifoldId;

use super::centroid::Centroid;
use super::{Color, PerPiece, PerSticker, Piece};
use crate::{Facet, PerFacet};

#[derive(Debug, Clone)]
pub struct Mesh {
    ndim: u8,

    /// Number of sticker colors in the mesh.
    pub color_count: usize,

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
    /// Polygon ID for each vertex.
    pub polygon_ids: Vec<u32>,

    /// Color ID for each polygon.
    pub color_ids: Vec<Color>,

    /// Centroid for each piece, used to apply piece explode.
    pub piece_centroids: Vec<f32>,
    /// Centroid for each facet, used to apply facet shrink.
    pub facet_centroids: Vec<f32>,

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
    pub fn new_empty(ndim: u8) -> Self {
        Mesh {
            ndim,
            color_count: 0,

            vertex_positions: vec![],
            u_tangents: vec![],
            v_tangents: vec![],
            sticker_shrink_vectors: vec![],
            piece_ids: vec![],
            facet_ids: vec![],
            polygon_ids: vec![],

            color_ids: vec![],

            piece_centroids: vec![],
            facet_centroids: vec![],

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
}

#[derive(Debug)]
pub(super) struct MeshBuilder {
    mesh: Mesh,

    piece_centroids: PerPiece<Vector>,
    facet_centroids: PerFacet<Centroid>,
    manifold_to_facet: HashMap<ManifoldId, Facet>,
    next_polygon_id: u32,
}
impl MeshBuilder {
    pub(super) fn new(ndim: u8) -> Self {
        MeshBuilder {
            mesh: Mesh::new_empty(ndim),

            piece_centroids: PerPiece::new(),
            facet_centroids: PerFacet::new(),
            manifold_to_facet: HashMap::new(),
            next_polygon_id: 1, // polygon ID 0 is reserved
        }
    }

    pub(super) fn add_color(&mut self) {
        self.mesh.color_count += 1;
    }

    pub(super) fn add_piece(&mut self, centroid_point: Vector) -> Result<MeshPieceBuilder<'_>> {
        let id = self.piece_centroids.push(centroid_point)?;
        Ok(MeshPieceBuilder { mesh: self, id })
    }

    pub(super) fn manifold_to_facet(&mut self, manifold_id: ManifoldId) -> Result<Facet> {
        Ok(match self.manifold_to_facet.entry(manifold_id) {
            hash_map::Entry::Occupied(e) => *e.get(),
            hash_map::Entry::Vacant(e) => *e.insert(self.facet_centroids.push(Centroid::ZERO)?),
        })
    }

    fn facet_centroid_mut(&mut self, facet: Facet) -> Option<&mut Centroid> {
        if facet == Facet::NONE {
            None
        } else {
            self.facet_centroids.extend_to_contain(facet).ok()?;
            Some(&mut self.facet_centroids[facet])
        }
    }

    pub(super) fn finish(mut self) -> Mesh {
        let ndim = self.mesh.ndim;
        for piece_centroid in self.piece_centroids.iter_values() {
            self.mesh
                .piece_centroids
                .extend(iter_f32(ndim, piece_centroid));
            println!("piece centroid = {piece_centroid}");
        }

        self.mesh
    }
}

#[derive(Debug)]
pub(super) struct MeshPieceBuilder<'a> {
    mesh: &'a mut MeshBuilder,
    id: Piece,
}
impl<'a> MeshPieceBuilder<'a> {
    pub(super) fn add_sticker<'b>(
        &'b mut self,
        surface_manifold: ManifoldId,
        color: Color,
        centroid: Centroid,
    ) -> Result<MeshStickerBuilder<'a, 'b>>
    where
        'a: 'b,
    {
        let surface = self.mesh.manifold_to_facet(surface_manifold)?;
        if let Some(surface_centroid) = self.mesh.facet_centroid_mut(surface) {
            *surface_centroid += centroid;
        }
        let index_range_start = self.mesh.mesh.triangles.len() as u32;
        Ok(MeshStickerBuilder {
            piece: self,
            facet: surface,
            color,
            index_range_start,
        })
    }
}
impl Drop for MeshPieceBuilder<'_> {
    fn drop(&mut self) {
        self.mesh
            .mesh
            .piece_internals_index_ranges
            .push(0..0) // TODO: internals index range
            .unwrap(); // TODO: unwrap icky
    }
}

#[derive(Debug)]
pub(super) struct MeshStickerBuilder<'a: 'b, 'b> {
    piece: &'b mut MeshPieceBuilder<'a>,
    facet: Facet,
    color: Color,
    index_range_start: u32,
}
impl<'a: 'b, 'b> MeshStickerBuilder<'a, 'b> {
    pub(super) fn add_polygon<'c>(
        &'c mut self,
        manifold: &'c Blade,
        color: Color,
    ) -> Result<MeshPolygonBuilder<'a, 'b, 'c>> {
        let id = self.piece.mesh.next_polygon_id;
        self.piece.mesh.next_polygon_id = id.checked_add(1).context("too many polygons")?;
        self.piece.mesh.mesh.color_ids.push(color);
        let tangent_space = manifold.opns_tangent_space();
        Ok(MeshPolygonBuilder {
            sticker: self,
            id,
            tangent_space,
        })
    }
}
impl Drop for MeshStickerBuilder<'_, '_> {
    fn drop(&mut self) {
        let mesh = &mut self.piece.mesh.mesh;
        let index_range_end = mesh.triangles.len() as u32;
        let _ = mesh
            .sticker_index_ranges
            .push(self.index_range_start..index_range_end);
    }
}

#[derive(Debug)]
pub(super) struct MeshPolygonBuilder<'a, 'b, 'c> {
    sticker: &'c mut MeshStickerBuilder<'a, 'b>,
    id: u32,
    tangent_space: TangentSpace,
}
impl MeshPolygonBuilder<'_, '_, '_> {
    pub(super) fn add_vertex(
        &mut self,
        pos: impl VectorRef,
        sticker_shrink_vector: impl VectorRef,
    ) -> Result<u32> {
        let mesh = &mut self.sticker.piece.mesh.mesh;
        let ndim = mesh.ndim();

        let vertex_id = mesh.vertex_count() as u32;

        mesh.vertex_positions.extend(iter_f32(ndim, &pos));
        let tangents = self.tangent_space.at(pos).context("bad tangent space")?;
        ensure!(tangents.len() == 2, "tangent space must be 2D");
        mesh.u_tangents.extend(iter_f32(ndim, &tangents[0]));
        mesh.v_tangents.extend(iter_f32(ndim, &tangents[1]));
        mesh.sticker_shrink_vectors
            .extend(iter_f32(ndim, &sticker_shrink_vector));
        mesh.piece_ids.push(self.sticker.piece.id);
        mesh.facet_ids.push(self.sticker.facet);
        mesh.polygon_ids.push(self.id);

        Ok(vertex_id)
    }
    pub(super) fn add_tri(&mut self, verts: [u32; 3]) {
        let mesh = &mut self.sticker.piece.mesh.mesh;
        mesh.triangles.push(verts);
    }
}

fn iter_f32<'v>(ndim: u8, v: &'v impl VectorRef) -> impl 'v + Iterator<Item = f32> {
    v.iter_ndim(ndim).map(|x| x as f32)
}
