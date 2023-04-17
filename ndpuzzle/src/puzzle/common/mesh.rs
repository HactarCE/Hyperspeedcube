// for whole puzzle:
// - vertex data (per sticker plus piece internals)
// -
//
// for each piece:
// - centroid (piece explode vector)
// - internal triangles
//     - vertex data range
//     - indices
//
// for each sticker:
// - surface triangles
//     - vertex data range
//     - indices
//
// for each facet:
// - centroid (facet shrink target)
//
//
// each vertex has:
// - point
// - surface tangents (2x)
// - sticker shrink vector

use ahash::{HashMap, HashMapExt, HashSet};
use anyhow::{ensure, Result};
use std::collections::hash_map::Entry;
use std::ops::Range;

use super::{Facet, PerPiece, PerSticker, Piece, Sticker};
use crate::collections::IndexNewtype;
use crate::geometry::{
    EuclideanCgaManifold, Manifold, ShapeArena, ShapeId, Simplexifier, VertexId,
};
use crate::math::{cga, Vector, VectorRef};

#[derive(Debug, Clone)]
pub struct Mesh {
    ndim: u8,
    vertex_count: usize,

    /// Coordinates for each vertex in N-dimensional space.
    pub vertex_positions: Vec<f32>,
    /// First tangent vector for each vertex, used to compute surface normal.
    pub u_tangents: Vec<f32>,
    /// Second tangent vector for each vertex, used to compute surface normal.
    pub v_tangents: Vec<f32>,
    /// Vector along which to move each vertex when applying sticker shrink.
    pub sticker_shrink_vectors: Vec<f32>,
    /// Polygon ID for each vertex.
    pub polygon_ids: Vec<u32>,
    /// Piece ID for each vertex.
    pub piece_ids: Vec<Piece>,
    /// Facet ID for each vertex.
    pub facet_ids: Vec<Facet>,

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

impl Mesh {
    pub fn new_empty(ndim: u8) -> Self {
        Mesh {
            ndim,
            vertex_count: 0,

            vertex_positions: vec![],
            u_tangents: vec![],
            v_tangents: vec![],
            sticker_shrink_vectors: vec![],
            polygon_ids: vec![],
            piece_ids: vec![],
            facet_ids: vec![],

            piece_centroids: vec![],
            facet_centroids: vec![],

            triangles: vec![],

            sticker_index_ranges: PerSticker::new(),
            piece_internals_index_ranges: PerPiece::new(),
        }
    }

    pub fn from_arena(
        arena: &ShapeArena<EuclideanCgaManifold>,
        facet_count: usize,
    ) -> Result<Self> {
        let ndim = arena.space().ndim()?;
        let mut mesh = Mesh::new_empty(ndim);

        let mut piece_id = Piece(0);
        let mut polygon_id = 0;

        let mut simplexifier = Simplexifier::new(arena);
        for &piece_shape in arena.roots() {
            let mut vertex_id_map: HashMap<VertexId, u32> = HashMap::new();

            let mut queue = vec![piece_shape];
            let mut seen: HashSet<ShapeId> = arena.roots().iter().copied().collect();
            let mut tris_per_sticker: Vec<Vec<[u32; 3]>> = vec![];
            let mut internal_tris: Vec<[u32; 3]> = vec![];
            while let Some(shape) = queue.pop() {
                match arena[shape].ndim()? {
                    0..=1 => continue,
                    3.. => {
                        // TODO: handle non-flat shapes
                        for b in arena[shape].boundary.iter() {
                            if seen.insert(b.id) {
                                queue.push(b.id);
                            }
                        }
                    }
                    2 => {
                        // TODO: stickers vs. internal pieces
                        tris_per_sticker.push(vec![]);
                        let tris = tris_per_sticker.last_mut().unwrap();

                        let tangent = arena[shape].manifold.tangent_space()?;
                        for tri in simplexifier.face_polygons(shape)? {
                            let [a, b, c] = tri.map(|v| match vertex_id_map.entry(v) {
                                Entry::Occupied(entry) => Ok(*entry.get()),
                                Entry::Vacant(entry) => {
                                    let vertex_position = &simplexifier[v];
                                    let tangents = tangent
                                        .basis_at(cga::Point::Finite(vertex_position.clone()))?;
                                    ensure!(tangents.len() == 2);
                                    let u_tangent = &tangents[0];
                                    let v_tangent = &tangents[1];
                                    let sticker_shrink_vector = Vector::EMPTY; // TODO: sticker shrink vector

                                    let new_id = mesh.add_vertex(
                                        vertex_position,
                                        u_tangent,
                                        v_tangent,
                                        sticker_shrink_vector,
                                        polygon_id,
                                        piece_id,
                                        Facet::MAX, // TODO: facet
                                    );

                                    entry.insert(new_id);
                                    Ok(new_id)
                                }
                            });
                            tris.push([a?, b?, c?]);
                            polygon_id += 1;
                        }
                    }
                }
            }

            let piece_centroid = simplexifier.shape_centroid(piece_shape)?;
            mesh.piece_centroids.extend(piece_centroid.iter_ndim(ndim));
            for sticker_tris in tris_per_sticker {
                let tri_range = mesh.add_tris(sticker_tris);
                mesh.sticker_index_ranges.push(tri_range)?;
            }
            let tri_range = mesh.add_tris(internal_tris);
            mesh.piece_internals_index_ranges.push(tri_range)?;

            piece_id = piece_id.next()?;
        }

        // TODO: real facet centroids
        for _ in 0..facet_count {
            mesh.facet_centroids.extend(Vector::EMPTY.iter_ndim(ndim));
        }

        Ok(mesh)
    }

    fn add_vertex(
        &mut self,
        vertex_position: impl VectorRef,
        u_tangent: impl VectorRef,
        v_tangent: impl VectorRef,
        sticker_shrink_vector: impl VectorRef,
        polygon_id: u32,
        piece_id: Piece,
        facet_id: Facet,
    ) -> u32 {
        let new_id = self.vertex_count as u32;
        self.vertex_count += 1;
        self.vertex_positions
            .extend(vertex_position.iter_ndim(self.ndim));
        self.u_tangents.extend(u_tangent.iter_ndim(self.ndim));
        self.v_tangents.extend(v_tangent.iter_ndim(self.ndim));
        self.sticker_shrink_vectors
            .extend(sticker_shrink_vector.iter_ndim(self.ndim));
        self.polygon_ids.push(polygon_id);
        self.piece_ids.push(piece_id);
        self.facet_ids.push(facet_id);
        new_id
    }

    fn add_tris(&mut self, tris: impl IntoIterator<Item = [u32; 3]>) -> Range<u32> {
        let start = self.triangles.len() as u32;
        self.triangles.extend(tris);
        let end = self.triangles.len() as u32;
        start..end
    }

    /// Returns the number of dimensions of the mesh.
    pub fn ndim(&self) -> u8 {
        self.ndim
    }
    /// Returns the number of vertices in the mesh.
    pub fn vertex_count(&self) -> usize {
        self.vertex_count
    }
    /// Returns the number of pieces in the mesh.
    pub fn piece_count(&self) -> usize {
        self.piece_internals_index_ranges.len() / self.ndim as usize
    }
    /// Returns the number of stickers in the mesh.
    pub fn sticker_count(&self) -> usize {
        self.sticker_index_ranges.len() / self.ndim as usize
    }

    /// Constructs an example mesh with 4 pieces that together form an
    /// octahedron.
    pub fn new_example_mesh() -> Result<Self> {
        let ndim = 3;
        let mut mesh = Mesh::new_empty(ndim);

        let x = Vector::unit(0);
        let y = Vector::unit(1);
        let z1 = Vector::unit(2);
        let z2 = -Vector::unit(2);
        let mut piece_id = Piece(0);
        let mut polygon_id = 0;
        for (u, v) in [
            (x.clone(), y.clone()),
            (x.clone(), -&y),
            (-&x, y.clone()),
            (-&x, -&y),
        ] {
            let i = mesh.triangles.len() as u32;

            let piece_centroid = (&u + &v) * 0.3;
            mesh.piece_centroids.extend(piece_centroid.iter_ndim(ndim));
            for (a, b, c, facet_id) in [
                (&u, &v, &z1, Facet(0)),
                (&u, &v, &z2, Facet(1)),
                (&u, &z1, &z2, Facet::MAX),
                (&v, &z1, &z2, Facet::MAX),
            ] {
                let u_tangent = (c - a).normalize().unwrap();
                let v_tangent = (c - b).normalize().unwrap();
                let sticker_shrink_target = (a + b + c) / 3.0;
                let triangle = [a, b, c].map(|vertex_position| {
                    let sticker_shrink_vector = &sticker_shrink_target - &vertex_position;
                    mesh.add_vertex(
                        vertex_position,
                        u_tangent.clone(),
                        v_tangent.clone(),
                        sticker_shrink_vector,
                        polygon_id,
                        piece_id,
                        facet_id,
                    )
                });
                mesh.triangles.push(triangle);
                polygon_id += 1;
            }

            mesh.sticker_index_ranges.push(i..(i + 1))?;
            mesh.sticker_index_ranges.push((i + 1)..(i + 2))?;
            mesh.piece_internals_index_ranges.push((i + 2)..(i + 4))?;
            piece_id = piece_id.next()?;
        }

        mesh.facet_centroids.extend(z1.iter_ndim(ndim));
        mesh.facet_centroids.extend(z2.iter_ndim(ndim));

        Ok(mesh)
    }
}

// /// Vertex data for a mesh.
// ///
// /// The `Vec<f32>` in this struct are all flattened arrays of N-dimensional
// /// vectors.
// pub struct MeshVertexData {
//     ndim: u8,
// }
// impl MeshVertexData {
//     fn push_vertex(&mut self, vertex: Vertex) {
//         self.points.extend(vertex.point.iter_ndim(self.ndim));

//         self.u_tangents
//             .extend(vertex.u_tangent.iter_ndim(self.ndim));
//         self.v_tangents
//             .extend(vertex.v_tangent.iter_ndim(self.ndim));

//         self.sticker_shrink_vectors
//             .extend(vertex.sticker_shrink_vector.iter_ndim(self.ndim));

//         self.facet_ids.push(vertex.facet_id);
//         self.piece_ids.push(vertex.piece_id);
//     }
//     fn push_tri(&mut self, indices: [u32; 3]) {
//         self.indices.extend(indices)
//     }

//     /// Returns the number of dimensions of the points in the mesh.
//     pub fn ndim(&self) -> u8 {
//         self.ndim
//     }
//     /// Returns the number of vertices in the mesh.
//     pub fn num_verts(&self) -> usize {
//         self.points.len() / self.ndim as usize
//     }
//     /// Returns the number of triangles in the mesh.
//     pub fn num_tris(&self) -> usize {
//         self.indices.len() / 3
//     }
// }
