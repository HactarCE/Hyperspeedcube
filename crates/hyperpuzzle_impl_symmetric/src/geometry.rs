use std::collections::HashMap;

use eyre::{Result, ensure};
use hypermath::prelude::*;
use hyperpuzzle_core::prelude::*;
use hypuz_util::FloatMinMaxIteratorExt;
use itertools::Itertools;

hypuz_util::typed_index_struct! {
    /// Vertex ID within a `PolytopeGeometry`.
    pub struct Vertex(pub u32);
}

/// List of a vector for each [`Vertex`], stored as a flattened
/// `Vec<`[`Float`]`>`.
pub type FlatVectorList = FlatTiVec<Vertex, Float>;

fn direct_product_vector_lists(a: &FlatVectorList, b: &FlatVectorList) -> FlatVectorList {
    FlatVectorList::from_iter(
        a.column_count() + b.column_count(),
        itertools::iproduct!(a.iter_rows(), b.iter_rows())
            .map(|(va, vb)| std::iter::chain(va, vb).copied()),
    )
}

/// Vertices, edges, and faces (polygons) of a polytope.
///
/// Higher-dimension elements are not stored because they are never actually
/// displayed.
#[derive(Debug)]
pub struct PolytopeGeometry {
    /// Vertex coordinates.
    pub verts: FlatVectorList,
    /// Vertex shrink vectors.
    pub shrink_vectors: FlatVectorList,
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
        verts: FlatVectorList::with_zero_columns(1), // 1 point
        shrink_vectors: FlatVectorList::with_zero_columns(1), // 1 vector
        edges: vec![],
        polygon_verts: vec![],
        polygon_sizes: vec![],
        centroid: Centroid::ZERO,
    };

    /// Returns the number of dimensions of the space containing the polytope.
    ///
    /// This may be larger than the number of dimensions of the polytope itself.
    /// In fact, this type has no knowldege of the number of dimensions of the
    /// polytope itself.
    pub fn space_ndim(&self) -> u8 {
        self.verts.column_count() as u8
    }

    /// Returns the number of vertices in the polytope.
    pub fn vertex_count(&self) -> usize {
        self.verts.row_count()
    }

    /// Returns the direct product of two polytopes `a` and `b`, which exists in
    /// a space of dimension `a.space_ndim() + b.space_ndim()`.
    pub fn direct_product(a: &Self, b: &Self) -> Self {
        let product_verts = |va: Vertex, vb: Vertex| Vertex(va.0 * b.vertex_count() as u32 + vb.0);
        let product_edges = |[va1, va2]: [Vertex; 2], [vb1, vb2]: [Vertex; 2]| {
            [
                product_verts(va1, vb1),
                product_verts(va2, vb1),
                product_verts(va1, vb2),
                product_verts(va2, vb2),
            ]
        };

        let verts = direct_product_vector_lists(&a.verts, &b.verts);
        let shrink_vectors = direct_product_vector_lists(&a.shrink_vectors, &b.shrink_vectors);

        let edges = if a.space_ndim() == 1
            && b.space_ndim() == 1
            && let &[ea] = a.edges.as_slice()
            && let &[eb] = b.edges.as_slice()
        {
            // Special case to keep orientation counterclockwise around
            // the polygon
            assert_eq!(a.vertex_count(), 2);
            assert_eq!(b.vertex_count(), 2);
            let [v0, v1, v2, v3] = product_edges(ea, eb);
            vec![[v0, v1], [v1, v3], [v3, v2], [v2, v0]]
        } else {
            std::iter::chain(
                // a edge * b vertex
                itertools::iproduct!(&a.edges, b.verts.iter_keys())
                    .map(|(&[va1, va2], vb)| [product_verts(va1, vb), product_verts(va2, vb)]),
                // a vertex * b edge
                itertools::iproduct!(a.verts.iter_keys(), &b.edges)
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
            itertools::iproduct!(a.verts.iter_keys(), &b.polygon_verts)
                .map(|(va, &pvb)| product_verts(va, pvb)),
            // b vertex * a polygon vertex
            itertools::iproduct!(b.verts.iter_keys(), &a.polygon_verts)
                .map(|(vb, &pva)| product_verts(pva, vb)),
        )
        .collect();

        let polygon_sizes = itertools::chain!(
            // a edge * b edge * 4
            itertools::iproduct!(0..a.edges.len(), 0..b.edges.len()).map(|_| 4),
            // a vertex * b polygon vertex
            itertools::iproduct!(a.verts.iter_keys(), &b.polygon_sizes).map(|(_, &size)| size),
            // b vertex * a polygon vertex
            itertools::iproduct!(b.verts.iter_keys(), &a.polygon_sizes).map(|(_, &size)| size),
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
            shrink_vectors,
            edges,
            polygon_verts,
            polygon_sizes,
            centroid,
        }
    }

    /// Constructs a `PolytopeGeometry` from a `hypershape` polytope element.
    pub fn from_polytope_element(
        polytope: hypershape::Element<'_>,
        sticker_shrink_vectors: &HashMap<hypershape::VertexId, Vector>,
    ) -> Result<Self> {
        let ndim = polytope.space().ndim();

        let mut vertex_map = HashMap::new();
        let mut verts = FlatVectorList::new(ndim as usize);
        let mut shrink_vectors = FlatVectorList::new(ndim as usize);
        for v in polytope.vertex_set() {
            vertex_map.insert(v.id(), verts.push_row(v.pos().as_vector().iter())?);
            let shrink_vector = sticker_shrink_vectors
                .get(&v.id())
                .unwrap_or(const { &Vector::EMPTY });
            shrink_vectors.push_row(shrink_vector.iter())?;
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
            shrink_vectors,
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

        // Add polygons.
        let mut i = 0;
        for &polygon_size in &self.polygon_sizes {
            let j = i + polygon_size;

            let vertices = self.polygon_verts[i..j]
                .iter()
                .map(|&v| (&self.verts[v], &self.shrink_vectors[v]))
                .collect_vec();

            // Calculate tangent vectors (and orientation in 2D & 3D).
            let [a, b, c] = [0, 1, 2].map(|n| Point::from(&vertices[n].0));
            ensure!(vertices.len() >= 3, "mesh polygon is too small");
            let [u, v] = hypermath::util::triangle_tangent_vectors(
                [&a, &b, &c],
                (ndim == 2 || ndim == 3).then_some(interior_point),
            )
            .unwrap_or_default(); // give up and return zero

            mesh.add_puzzle_polygon(vertices, piece_id, surface_id, &u, &v)?;

            i = j;
        }

        // Add edges if there are no polygons.
        if self.polygon_sizes.is_empty() {
            let polygon_id = mesh.next_polygon_id()?; // dummy polygon ID
            let vertex_offset = mesh.vertex_count() as u32;
            for (point, sticker_shrink_vector) in std::iter::zip(
                self.verts.iter_rows().map(Point::from),
                self.shrink_vectors.iter_rows(),
            ) {
                mesh.add_puzzle_vertex(MeshVertexData {
                    position: &point,
                    u_tangent: &Vector::EMPTY,
                    v_tangent: &Vector::EMPTY,
                    sticker_shrink_vector,
                    piece_id,
                    surface_id,
                    polygon_id,
                })?;
            }
            for &edge in &self.edges {
                mesh.edges.push(edge.map(|Vertex(i)| vertex_offset + i));
            }
        }

        let end = mesh.counts();
        Ok(MeshRange::new(start, end))
    }

    /// Returns the minimum and maximum heights of vertices in the polytope
    /// along the given axis vector.
    ///
    /// Returns `None` if the polytope is empty.
    pub(super) fn height_on_axis(&self, axis: &Vector) -> Option<(f64, f64)> {
        self.verts
            .iter_rows()
            .map(|v| std::iter::zip(axis.iter(), v).map(|(a, b)| a * b).sum()) // dot product
            .minmax_float()
            .into_option()
    }
}

impl TransformByMotor for PolytopeGeometry {
    fn transform_by(&self, m: &pga::Motor) -> Self {
        let ndim = self.space_ndim();

        let mut verts = FlatVectorList::with_capacity(ndim as usize, self.vertex_count());
        for (_, p) in &self.verts {
            verts
                .push_row(m.transform_point(p).as_vector().iter())
                .expect("error transforming geometry");
        }
        let mut shrink_vectors = FlatVectorList::with_capacity(ndim as usize, self.vertex_count());
        for (_, v) in &self.shrink_vectors {
            shrink_vectors
                .push_row(m.transform_vector(v).iter())
                .expect("error transforming geometry");
        }

        PolytopeGeometry {
            verts,
            shrink_vectors,
            edges: self.edges.clone(),
            polygon_verts: self.polygon_verts.clone(),
            polygon_sizes: self.polygon_sizes.clone(),
            centroid: m.transform(&self.centroid),
        }
    }
}
