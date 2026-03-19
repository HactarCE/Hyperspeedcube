use std::{
    collections::HashMap,
    fmt,
    marker::PhantomData,
    ops::{Index, Range},
};

use eyre::Result;
use hypermath::prelude::*;
use hyperpuzzle_core::prelude::*;
use itertools::Itertools;

use crate::direct_product_points;

hypuz_util::typed_index_struct! {
    /// Vertex ID.
    pub struct Vertex(pub u32);
}

#[derive(Debug)]
pub struct PieceGeometry {
    pub polytope: PolytopeGeometry,
    pub centroid: Point,
    pub facets: Vec<PieceFacetGeometry>,
}

impl PieceGeometry {
    pub fn direct_product(a: &Self, b: &Self, a_surfaces: usize) -> Self {
        let ndim = a.polytope.ndim() + b.polytope.ndim();

        Self {
            polytope: PolytopeGeometry::direct_product(&a.polytope, &b.polytope),
            centroid: direct_product_points(
                a.polytope.ndim(),
                b.polytope.ndim(),
                &a.centroid,
                &b.centroid,
            ),
            facets: std::iter::chain(
                a.facets
                    .iter()
                    .filter(|f| ndim <= 3 || f.sticker_data.is_some()) // remove internals in 4D+
                    .map(|a_facet| PieceFacetGeometry {
                        polytope: PolytopeGeometry::direct_product(&a_facet.polytope, &b.polytope),
                        sticker_data: a_facet.sticker_data.as_ref().map(|sticker_data| {
                            StickerData {
                                surface: sticker_data.surface,
                            }
                        }),
                    }),
                b.facets
                    .iter()
                    .filter(|f| ndim <= 3 || f.sticker_data.is_some())
                    .map(|b_facet| PieceFacetGeometry {
                        polytope: PolytopeGeometry::direct_product(&a.polytope, &b_facet.polytope),
                        sticker_data: b_facet.sticker_data.as_ref().map(|sticker_data| {
                            StickerData {
                                surface: Surface(a_surfaces as u16 + sticker_data.surface.0),
                            }
                        }),
                    }),
            )
            .collect(),
        }
    }
}

#[derive(Debug)]
pub struct PieceFacetGeometry {
    pub polytope: PolytopeGeometry,
    pub sticker_data: Option<StickerData>,
}

#[derive(Debug)]
pub struct StickerData {
    pub surface: Surface,
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

pub struct PointPer<I> {
    ndim: u8,
    values: Vec<Float>,
    _marker: PhantomData<I>,
}

impl<I> fmt::Debug for PointPer<I> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list()
            .entries(self.values.chunks_exact(self.ndim as usize))
            .finish()
    }
}

impl<I: TypedIndex> Index<I> for PointPer<I> {
    type Output = [Float];

    fn index(&self, index: I) -> &Self::Output {
        &self.values[self.index_range(index)]
    }
}

impl<'a, I: TypedIndex> IntoIterator for &'a PointPer<I> {
    type Item = &'a [Float];

    type IntoIter = std::slice::ChunksExact<'a, Float>;

    fn into_iter(self) -> Self::IntoIter {
        self.values.chunks_exact(self.ndim as usize)
    }
}

impl<I: TypedIndex> PointPer<I> {
    pub fn new(ndim: u8) -> Self {
        Self {
            ndim,
            values: vec![],
            _marker: PhantomData,
        }
    }

    pub fn from_iter(
        ndim: u8,
        vectors: impl IntoIterator<Item = Vector>,
    ) -> Result<Self, IndexOverflow> {
        let mut ret = Self::new(ndim);
        for p in vectors {
            ret.push(p)?;
        }
        Ok(ret)
    }

    fn direct_product(a: &Self, b: &Self) -> Self {
        Self {
            ndim: a.ndim + b.ndim,
            values: itertools::iproduct!(a, b)
                .flat_map(|(va, vb)| std::iter::chain(va, vb).copied())
                .collect(),
            _marker: PhantomData,
        }
    }

    pub fn get(&self, i: I) -> Point {
        Point::from(&self[i])
    }

    fn index_range(&self, v: I) -> Range<usize> {
        let i = v.to_index();
        let ndim = self.ndim as usize;
        i * ndim..(i + 1) * ndim
    }

    pub fn len(&self) -> usize {
        self.values.len() / self.ndim as usize
    }

    pub fn push(&mut self, point: impl VectorRef) -> Result<I, IndexOverflow> {
        let len = self.len();
        self.values.extend(point.iter_ndim(self.ndim));
        I::try_from_index(len)
    }

    pub fn keys(&self) -> TypedIndexIter<I> {
        I::iter(self.len())
    }
}

/// Vertices, edges, and triangles of a polytope.
#[derive(Debug)]
pub struct PolytopeGeometry {
    /// Vertex coordinates.
    pub verts: PointPer<Vertex>,
    /// Edges, defined in terms of `verts`.
    pub edges: Vec<[Vertex; 2]>,
    /// Concatenated polygons, defined in terms of `verts`.
    ///
    /// To reconstruct the original polygons, split them using
    /// `polygon_vertex_counts`.
    ///
    /// The vertices within each polygon are in order (whether that order is
    /// counterclockwise or clockwise is undefined).
    pub polygon_verts: Vec<Vertex>,
    /// Number of vertices in `polygon_verts` for each polygon. This can be used
    /// to split `polygon_verts` into a range of vertices for each polygon.
    pub polygon_sizes: Vec<usize>,
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

        let verts = PointPer::direct_product(&a.verts, &b.verts);

        let edges = if a.ndim() == 1
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

        Self {
            verts,
            edges,
            polygon_verts,
            polygon_sizes,
        }
    }

    /// Constructs geometry for a polytope element.
    ///
    /// The centroid is used to orient elements in 2D and 3D.
    pub fn from_polytope_element(polytope: hypershape::Element<'_>) -> Result<Self> {
        let ndim = polytope.space().ndim();

        let mut vertex_map = HashMap::new();
        let mut verts = PointPer::new(polytope.space().ndim());
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

        Ok(Self {
            verts,
            edges,
            polygon_verts,
            polygon_sizes,
        })
    }
}
