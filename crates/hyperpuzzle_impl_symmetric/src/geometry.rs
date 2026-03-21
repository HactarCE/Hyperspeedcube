use std::collections::{HashMap, hash_map};
use std::fmt;
use std::ops::{Index, Range};

use eyre::{Result, ensure};
use hypermath::prelude::*;
use hyperpuzzle_core::prelude::*;
use itertools::Itertools;

hypuz_util::typed_index_struct! {
    /// Vertex ID within a `PolytopeGeometry`.
    pub struct Vertex(pub u32);
}

/// List of unique vertices, indexed [`Vertex`].
pub struct VertexList {
    /// Number of dimensions.
    ndim: u8,
    /// Number of points.
    ///
    /// This is tracked separately from `values` so that it works correctly in
    /// the zero-dimensional case.
    len: usize,
    /// Flattened list of coordinates for each vertex.
    values: Vec<Float>,
}

impl fmt::Debug for VertexList {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.iter()).finish()
    }
}

impl Index<Vertex> for VertexList {
    type Output = [Float];

    fn index(&self, index: Vertex) -> &Self::Output {
        &self.values[self.index_range(index)]
    }
}

impl VertexList {
    pub fn new(ndim: u8) -> Self {
        Self {
            ndim,
            len: 0,
            values: vec![],
        }
    }

    fn direct_product(a: &Self, b: &Self) -> Self {
        Self {
            ndim: a.ndim + b.ndim,
            len: a.len * b.len,
            values: itertools::iproduct!(a.iter(), b.iter())
                .flat_map(|(va, vb)| std::iter::chain(va, vb).copied())
                .collect(),
        }
    }

    pub fn get(&self, i: Vertex) -> Point {
        Point::from(&self[i])
    }

    fn index_range(&self, v: Vertex) -> Range<usize> {
        let i = v.to_index();
        let ndim = self.ndim as usize;
        i * ndim..(i + 1) * ndim
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn push(&mut self, point: impl VectorRef) -> Result<Vertex, IndexOverflow> {
        let len = self.len;
        self.len += 1;
        self.values.extend(point.iter_ndim(self.ndim));
        Vertex::try_from_index(len)
    }

    pub fn keys(&self) -> TypedIndexIter<Vertex> {
        Vertex::iter(self.len)
    }

    pub fn iter(&self) -> impl Clone + DoubleEndedIterator<Item = &[Float]> + ExactSizeIterator {
        self.keys().map(|i| &self[i])
    }
}

/// Vertices, edges, and triangles of a polytope.
///
/// Higher-dimension elements are not stored because they are never actually
/// displayed.
#[derive(Debug)]
pub struct PolytopeGeometry {
    /// Vertex coordinates.
    pub verts: VertexList,
    /// Edges, defined in terms of `verts`.
    pub edges: Vec<[Vertex; 2]>,
    /// Concatenated polygons, defined in terms of `verts`.
    ///
    /// To reconstruct the original polygons, split them using
    /// `polygon_vertex_counts`.
    ///
    /// The vertices within each polygon are in order. (It is undefined whether
    /// that order is counterclockwise or clockwise.)
    pub polygon_verts: Vec<Vertex>,
    /// Number of vertices in `polygon_verts` for each polygon. This can be used
    /// to split `polygon_verts` into a range of vertices for each polygon.
    pub polygon_sizes: Vec<usize>,
    /// Centroid and Lebasgue measure of the polytope.
    pub centroid: Centroid,
}

impl PolytopeGeometry {
    /// Geometry for a single point in a zero-dimensional space.
    pub const POINT: Self = Self {
        verts: VertexList {
            ndim: 0,
            len: 1,
            values: vec![],
        },
        edges: vec![],
        polygon_verts: vec![],
        polygon_sizes: vec![],
        centroid: Centroid::ZERO,
    };

    /// Retunrs the number of dimensions of the space containing the polytope.
    ///
    /// This may be larger than the number of dimensions of the polytope itself.
    /// In fact, this type has no knowldege of the number of dimensions of the
    /// polytope itself.
    pub fn space_ndim(&self) -> u8 {
        self.verts.ndim
    }

    /// Returns the direct product of two polytopes `a` and `b`, which exists in
    /// a space of dimension `a.space_ndim() + b.space_ndim()`.
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

        let verts = VertexList::direct_product(&a.verts, &b.verts);

        let edges = if a.space_ndim() == 1
            && b.space_ndim() == 1
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
                itertools::iproduct!(&a.edges, b.verts.keys())
                    .map(|(&[va1, va2], vb)| [product_verts(va1, vb), product_verts(va2, vb)]),
                // a vertex * b edge
                itertools::iproduct!(a.verts.keys(), &b.edges)
                    .map(|(va, &[vb1, vb2])| [product_verts(va, vb1), product_verts(va, vb2)]),
            )
            .collect()
        };

        let polygon_verts = itertools::chain!(
            // a edge * b edge * 4
            itertools::iproduct!(&a.edges, &b.edges).flat_map(|(&ea, &eb)| {
                let [v0, v1, v2, v3] = product_edges(ea, eb);
                [v0, v1, v3, v2]
            }),
            // a vertex * b polygon vertex
            itertools::iproduct!(a.verts.keys(), &b.polygon_verts)
                .map(|(va, &pvb)| product_verts(va, pvb)),
            // b vertex * a polygon vertex
            itertools::iproduct!(b.verts.keys(), &a.polygon_verts)
                .map(|(vb, &pva)| product_verts(pva, vb)),
        )
        .collect();

        let polygon_sizes = itertools::chain!(
            // a edge * b edge * 4
            itertools::iproduct!(0..a.edges.len(), 0..b.edges.len()).map(|_| 4),
            // a vertex * b polygon vertex
            itertools::iproduct!(a.verts.keys(), &b.polygon_sizes).map(|(_, &size)| size),
            // b vertex * a polygon vertex
            itertools::iproduct!(b.verts.keys(), &a.polygon_sizes).map(|(_, &size)| size),
        )
        .collect();

        let centroid = Centroid::new(
            &std::iter::chain(
                a.centroid.center().as_vector().iter_ndim(a.space_ndim()),
                b.centroid.center().as_vector().iter_ndim(b.space_ndim()),
            )
            .collect(),
            a.centroid.weight() + b.centroid.weight(),
        );

        Self {
            verts,
            edges,
            polygon_verts,
            polygon_sizes,
            centroid,
        }
    }

    /// Constructs a `PolytopeGeometry` from a `hypershape` polytope element.
    pub fn from_polytope_element(polytope: hypershape::Element<'_>) -> Result<Self> {
        let ndim = polytope.space().ndim();

        let mut vertex_map = HashMap::new();
        let mut verts = VertexList::new(polytope.space().ndim());
        for v in polytope.vertex_set() {
            vertex_map.insert(v.id(), verts.push(v.pos().into_vector())?);
        }

        let mut edges: Vec<[Vertex; 2]> = vec![];
        let mut polygon_verts: Vec<Vertex> = vec![];
        let mut polygon_sizes: Vec<usize> = vec![];
        if ndim == 2
            && let Ok(f) = polytope.as_face()
        {
            for edge in f.edges_in_order()? {
                let [v1, v2] = edge.map(|v| vertex_map[&v.id()]);
                edges.push([v1, v2]);
                polygon_verts.push(v1);
            }
            polygon_sizes.push(polygon_verts.len());
        } else {
            edges = polytope
                .edge_set()
                .map(|edge| eyre::Ok(edge.endpoints()?.map(|v| vertex_map[&v.id()])))
                .try_collect()?;

            for f in polytope.face_set() {
                let polygon_start = polygon_verts.len();
                polygon_verts.extend(f.vertices_in_order()?.map(|v| vertex_map[&v.id()]));
                let polygon_end = polygon_verts.len();
                polygon_sizes.push(polygon_end - polygon_start);
            }
        }

        let centroid = polytope.centroid()?;

        Ok(Self {
            verts,
            edges,
            polygon_verts,
            polygon_sizes,
            centroid,
        })
    }

    /// Adds a polytope to a mesh.
    ///
    /// `interior_point` is used for orienting triangles in 2D and 3D. When
    /// generating the mesh for a piece or sticker, `interior_point` must be a
    /// point somewhere on the interior of the **piece** (not sticker). For
    /// generating a 2D polytope in a 2D space, it must be a point with a
    /// negative Z coordinate.
    pub fn add_to_mesh(
        &self,
        mesh: &mut Mesh,
        surface_id: Surface,
        piece_id: Piece,
        interior_point: &Point,
    ) -> Result<MeshRange> {
        let ndim = self.space_ndim();

        let start = mesh.counts();

        // Add polygons and triangles.
        let dummy_polygon = mesh.next_polygon_id()?; // for edges with no polygon
        let mut vertex_map = HashMap::new();
        let mut i = 0;
        for &polygon_size in &self.polygon_sizes {
            let polygon_id_in_mesh = mesh.next_polygon_id()?;

            let j = i + polygon_size as usize;
            let polygon = &self.polygon_verts[i..j];

            // Calculate tangent vectors.
            ensure!(polygon.len() >= 3, "mesh polygon is too small");
            let [a, b, c] = [0, 1, 2].map(|n| self.verts.get(polygon[n]));
            // IIFE to mimic try_block
            let (mut u_tangent, mut v_tangent) = (|| {
                let u = (b - &a).normalize()?;
                let v = (c - &a).rejected_from(&u)?.normalize()?;
                Some((u, v))
            })()
            .unwrap_or_default(); // give up and return zero

            // Fix polygon orientation in 2D and 3D.
            if ndim == 2 || ndim == 3 {
                if u_tangent
                    .cross_product_3d(&v_tangent)
                    .dot(a - interior_point)
                    .is_sign_negative()
                {
                    std::mem::swap(&mut u_tangent, &mut v_tangent);
                }
            }

            let polygon_start = mesh.vertex_count() as u32;
            for &vertex_id in polygon {
                let vertex_id_in_mesh = mesh.add_puzzle_vertex(MeshVertexData {
                    position: &self.verts.get(vertex_id),
                    u_tangent: &u_tangent,
                    v_tangent: &v_tangent,
                    sticker_shrink_vector: &Vector::zero(0), // TODO
                    piece_id,
                    surface_id,
                    polygon_id: polygon_id_in_mesh,
                })?;
                vertex_map.entry(vertex_id).or_insert(vertex_id_in_mesh);
            }

            for k in 2..polygon_size as u32 {
                mesh.triangles
                    .push([0, k - 1, k].map(|q| polygon_start + q));
            }

            i = j;
        }

        // Add edges.
        let mut get_or_add_vertex_for_edge = |mesh: &mut Mesh, v| {
            eyre::Ok(match vertex_map.entry(v) {
                hash_map::Entry::Occupied(entry) => *entry.get(),
                hash_map::Entry::Vacant(entry) => {
                    *entry.insert(mesh.add_puzzle_vertex(MeshVertexData {
                        position: &self.verts.get(v),
                        u_tangent: &Vector::EMPTY,
                        v_tangent: &Vector::EMPTY,
                        sticker_shrink_vector: &Vector::zero(0), // TODO
                        piece_id,
                        surface_id,
                        polygon_id: dummy_polygon,
                    })?)
                }
            })
        };
        for &[v1, v2] in &self.edges {
            let v1 = get_or_add_vertex_for_edge(mesh, v1)?;
            let v2 = get_or_add_vertex_for_edge(mesh, v2)?;
            mesh.edges.push([v1, v2]);
        }

        let end = mesh.counts();
        Ok(MeshRange::new(start, end))
    }
}
