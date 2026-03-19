use std::{
    collections::HashMap,
    fmt,
    ops::{Index, Range},
};

use eyre::Result;
use hypermath::prelude::*;
use hyperpuzzle_core::prelude::*;
use itertools::Itertools;

use crate::direct_product_points;

hypuz_util::typed_index_struct! {
    pub struct Vertex(pub u32);
}

#[derive(Debug)]
pub struct PieceGeometry {
    pub polytope: PolytopeGeometry,
    pub centroid: Point,
}

impl PieceGeometry {
    pub fn direct_product(a: &Self, b: &Self) -> Self {
        Self {
            polytope: PolytopeGeometry::direct_product(&a.polytope, &b.polytope),
            centroid: direct_product_points(
                a.polytope.ndim(),
                b.polytope.ndim(),
                &a.centroid,
                &b.centroid,
            ),
        }
    }
}

#[derive(Debug)]
pub struct StickerGeometry {
    pub polytope: PolytopeGeometry,
    pub surface: Surface,
}

impl StickerGeometry {
    pub fn direct_product_sticker_piece(a: &Self, b: &PieceGeometry) -> Self {
        Self {
            polytope: PolytopeGeometry::direct_product(&a.polytope, &b.polytope),
            surface: a.surface,
        }
    }

    pub fn direct_product_piece_sticker(
        a: &PieceGeometry,
        b: &Self,
        a_surface_count: usize,
    ) -> Self {
        Self {
            polytope: PolytopeGeometry::direct_product(&a.polytope, &b.polytope),
            surface: Surface(a_surface_count as u16 + b.surface.0),
        }
    }
}

#[derive(Debug)]
pub struct SurfaceGeometry {
    pub ndim: u8,
    pub centroid: Point,
    pub normal: Vector,
}

impl SurfaceGeometry {
    pub fn lift_by_ndim(&self, ndim_below: u8, ndim_above: u8) -> Self {
        let below = std::iter::repeat_n(0.0, ndim_below as usize);
        let above = std::iter::repeat_n(0.0, ndim_above as usize);
        Self {
            ndim: ndim_below + self.ndim + ndim_above,
            centroid: itertools::chain!(
                below.clone(),
                self.centroid.as_vector().iter_ndim(self.ndim),
                above.clone()
            )
            .collect(),
            normal: itertools::chain!(
                below.clone(),
                self.normal.iter_ndim(self.ndim),
                above.clone()
            )
            .collect(),
        }
    }
}

pub struct VectorPerVertex {
    ndim: u8,
    values: Vec<Float>,
}

impl fmt::Debug for VectorPerVertex {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list()
            .entries(self.values.chunks_exact(self.ndim as usize))
            .finish()
    }
}

impl Index<Vertex> for VectorPerVertex {
    type Output = [Float];

    fn index(&self, index: Vertex) -> &Self::Output {
        &self.values[self.index_range(index)]
    }
}

impl<'a> IntoIterator for &'a VectorPerVertex {
    type Item = &'a [Float];

    type IntoIter = std::slice::ChunksExact<'a, Float>;

    fn into_iter(self) -> Self::IntoIter {
        self.values.chunks_exact(self.ndim as usize)
    }
}

impl VectorPerVertex {
    fn direct_product(a: &Self, b: &Self) -> Self {
        Self {
            ndim: a.ndim + b.ndim,
            values: itertools::iproduct!(a, b)
                .flat_map(|(va, vb)| std::iter::chain(va, vb).copied())
                .collect(),
        }
    }

    fn index_range(&self, v: Vertex) -> Range<usize> {
        let i = v.0 as usize;
        let ndim = self.ndim as usize;
        i * ndim..(i + 1) * ndim
    }

    pub fn new(ndim: u8) -> Self {
        Self {
            ndim,
            values: vec![],
        }
    }

    pub fn from_iter(ndim: u8, vectors: impl IntoIterator<Item = Vector>) -> Self {
        let mut ret = Self::new(ndim);
        for p in vectors {
            ret.push(p);
        }
        ret
    }

    pub fn len(&self) -> usize {
        self.values.len() / self.ndim as usize
    }

    pub fn push(&mut self, point: impl VectorRef) -> Vertex {
        let len = self.len() as u32;
        self.values.extend(point.iter_ndim(self.ndim));
        Vertex(len as u32)
    }

    pub fn keys(&self) -> TypedIndexIter<Vertex> {
        Vertex::iter(self.len())
    }
}

/// Vertices, edges, and triangles of a polytope.
#[derive(Debug)]
pub struct PolytopeGeometry {
    /// Vertex coordinates.
    pub verts: VectorPerVertex,
    /// Edges, defined in terms of `verts`.
    ///
    /// - In 1D, the vertices within an edge are ordered from negative to
    ///   positive.
    /// - In 2D, the vertices within an edge are ordered counterclockwise around
    ///   the polygon.
    pub edges: Vec<[Vertex; 2]>,
    /// Triangles, defined in terms of `verts`.
    ///
    /// - In 2D, the vertices within a triangle are ordered counterclockwise.
    /// - In 3D, the vertices within a triangle are ordered counterclockwise
    ///   from the outside of the polygon.
    pub tris: Vec<[Vertex; 3]>,
}

impl PolytopeGeometry {
    pub fn ndim(&self) -> u8 {
        self.verts.ndim
    }

    pub fn direct_product(a: &Self, b: &Self) -> Self {
        let product_verts = |va: Vertex, vb: Vertex| Vertex(va.0 * b.verts.len() as u32 + vb.0);
        let product_edges = |[va1, va2]: [Vertex; 2], [vb1, vb2]: [Vertex; 2]| {
            [
                product_verts(va1, vb1),
                product_verts(va2, vb1),
                product_verts(va1, vb2),
                product_verts(va2, vb2),
            ]
        };

        Self {
            verts: VectorPerVertex::direct_product(&a.verts, &b.verts),

            edges: {
                if a.ndim() == 1
                    && b.ndim() == 1
                    && let &[ea] = a.edges.as_slice()
                    && let &[eb] = b.edges.as_slice()
                {
                    // Special case to keep orientation counterclockwise around
                    // the polygon
                    assert_eq!(a.verts.len(), 2);
                    assert_eq!(b.verts.len(), 2);
                    let [v0, v1, v2, v3] = product_edges(ea, eb);
                    vec![[v0, v1], [v1, v3], [v3, v2], [v2, v0]]
                } else {
                    std::iter::chain(
                        // a edge * b vertex
                        itertools::iproduct!(&a.edges, b.verts.keys()).map(|(&[va1, va2], vb)| {
                            [product_verts(va1, vb), product_verts(va2, vb)]
                        }),
                        // a vertex * b edge
                        itertools::iproduct!(a.verts.keys(), &b.edges).map(|(va, &[vb1, vb2])| {
                            [product_verts(va, vb1), product_verts(va, vb2)]
                        }),
                    )
                    .collect()
                }
            },

            tris: itertools::chain!(
                // a triangle * b vertex
                itertools::iproduct!(&a.tris, b.verts.keys())
                    .map(|(tri_a, vb)| tri_a.map(|va| product_verts(va, vb))),
                // a edge * b edge * 2
                itertools::iproduct!(&a.edges, &b.edges).flat_map(|(&ea, &eb)| {
                    let [v0, v1, v2, v3] = product_edges(ea, eb);
                    [[v0, v1, v2], [v3, v2, v1]]
                }),
                // a vertex * b triangle
                itertools::iproduct!(a.verts.keys(), &b.tris)
                    .map(|(va, tri_b)| tri_b.map(|vb| product_verts(va, vb))),
            )
            .collect(),
        }
    }

    pub fn from_polytope_element(polytope: hypershape::Element<'_>) -> Result<Self> {
        let mut vertex_map = HashMap::new();

        let mut verts = VectorPerVertex::new(polytope.space().ndim());
        for v in polytope.vertex_set() {
            vertex_map.insert(v.id(), verts.push(v.pos().into_vector()));
        }

        let edges = polytope
            .edge_set()
            .map(|e| eyre::Ok(e.endpoints()?.map(|v| vertex_map[&v.id()])))
            .try_collect()?;

        let tris = polytope
            .face_set()
            .map(|f| eyre::Ok(f.triangles()?))
            .flatten_ok()
            .map_ok(|tri| tri.map(|v| vertex_map[&v]))
            .try_collect()?;

        Ok(Self { verts, edges, tris })
    }
}
