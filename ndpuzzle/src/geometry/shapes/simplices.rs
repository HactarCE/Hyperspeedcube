use ahash::AHashMap;
use anyhow::{bail, ensure, Context, Result};
use float_ord::FloatOrd;
use itertools::Itertools;
use smallvec::{smallvec, SmallVec};
use std::fmt;
use std::{iter::Sum, ops::Index};
use tinyset::Set64;

use super::{Manifold, ShapeArena, ShapeId};
use crate::collections::VectorHashMap;
use crate::geometry::ManifoldSplit;
use crate::math::*;

/// How many times to recursively subdivide a sphere.
const DEFAULT_SPHERE_RESOLUTION: u8 = 2;

pub struct Simplexifier<'a> {
    arena: &'a ShapeArena,

    vertices: Vec<Vector>,
    vertex_ids: VectorHashMap<VertexId>,
    shape_simplices_cache: AHashMap<ShapeId, SimplexBlob>,
}
impl Index<VertexId> for Simplexifier<'_> {
    type Output = Vector;

    fn index(&self, index: VertexId) -> &Self::Output {
        &self.vertices[index.0 as usize]
    }
}
impl<'a> Simplexifier<'a> {
    pub fn new(arena: &'a ShapeArena) -> Self {
        Simplexifier {
            arena,

            vertices: vec![],
            vertex_ids: VectorHashMap::new(),
            shape_simplices_cache: AHashMap::new(),
        }
    }

    pub fn vertex_point(&self, v: VertexId) -> cga::Point {
        cga::Point::Finite(self[v].clone())
    }

    fn ensure_finite(&self, shape: ShapeId) -> Result<()> {
        ensure!(
            self.is_shape_finite(shape)?,
            "cannot generate mesh for infinite \
             shape! this is most likely a user \
             error (bad puzzle definition)",
        );
        Ok(())
    }
    fn is_shape_finite(&self, shape: ShapeId) -> Result<bool> {
        Ok(!self
            .arena
            .shape_contains_point(shape, &cga::Point::Infinity)?)
    }
    fn radius_upper_bound(&self, shape: ShapeId) -> Result<Float> {
        self.ensure_finite(shape)?;

        let mut max_radius_seen = 1.0;

        let mut seen = Set64::new();
        seen.insert(shape);
        let mut queue = vec![shape];
        while let Some(elem) = queue.pop() {
            let shape_manifold = &self.arena[elem].manifold;

            let r = if self.arena[elem].ndim()? == 0 {
                // Point pair
                let [a, b] = shape_manifold
                    .to_point_pair()?
                    .map(|p| p.to_finite().unwrap_or(Vector::EMPTY).mag2());
                Float::max(a, b).sqrt()
            } else if shape_manifold.is_flat() {
                // Flat object; use the max radius of the boundary shapes
                for child in self.arena[shape].boundary.iter() {
                    if seen.insert(child.id) {
                        queue.push(child.id)
                    }
                }
                continue;
            } else {
                // Round object; calculate max radius using center
                let ipns = shape_manifold.ipns();
                let center = ipns.ipns_sphere_center().to_finite()?;
                let radius = ipns
                    .ipns_radius()
                    .context("unable to get radius of hypersphere")?;
                center.mag() + radius
            };

            if r > max_radius_seen {
                max_radius_seen = r;
            }
        }

        Ok(max_radius_seen)
    }

    pub fn shape_centroid_point(&mut self, shape: ShapeId) -> Result<Vector> {
        let shape_manifold = &self.arena[shape].manifold;
        // Add up those centroids.
        let centroid = self.shape_centroid(shape)?;
        // Project the point back onto the manifold.
        shape_manifold
            .project_point(&cga::Point::Finite(centroid.com))?
            .context("unable to compute centroid of shape")?
            .to_finite()
    }
    pub fn shape_centroid(&mut self, shape: ShapeId) -> Result<Centroid> {
        let shape_manifold = &self.arena[shape].manifold;
        // Turn the shape into simplices.
        let simplices = self.shape_simplices(shape)?.0.into_iter();
        // Compute the centroid of each simplex.
        let centroids = simplices.map(|s| self.simplex_centroid(&s, &shape_manifold));
        // Add up those centroids.
        centroids.sum::<Result<Centroid>>()
    }

    fn simplex_centroid(&self, s: &Simplex, m: &Manifold) -> Result<Centroid> {
        let mut verts_iter = s.0.iter();
        let Some(v0) = verts_iter.next() else {
            return Ok(Centroid::default());
        };
        Ok(Centroid {
            blade: verts_iter.fold(cga::Blade::scalar(1.0), |b, v| {
                b ^ cga::Blade::vector(&self[v] - &self[v0])
            }),
            com: self.simplex_center(s, m)?,
        })
    }
    fn simplex_center(&self, s: &Simplex, m: &Manifold) -> Result<Vector> {
        let mut sum = Vector::EMPTY;
        for v in s.0.iter() {
            sum += &self[v];
        }
        let point = sum / s.0.len() as Float;
        if m.is_flat() {
            Ok(point)
        } else {
            m.project_point(&cga::Point::Finite(point))?
                .context("failed to project point onto manifold")?
                .to_finite()
        }
    }

    pub fn face_polygons(&mut self, shape: ShapeId) -> Result<Vec<[VertexId; 3]>> {
        // TODO: don't use this to generate the whole mesh because it doesn't
        // cover 3D+ non-flat surfaces.
        ensure!(self.arena[shape].ndim()? == 2);
        // TODO: Optimize for flat polygon bounded by flat elements.
        self.shape_simplices(shape)?
            .0
            .into_iter()
            .map(|s| s.try_into_array())
            .collect::<Option<Vec<[VertexId; 3]>>>()
            .context("simplex has wrong number of dimensions")
    }

    fn shape_simplices(&mut self, shape: ShapeId) -> Result<SimplexBlob> {
        if let Some(result) = self.shape_simplices_cache.get(&shape) {
            return Ok(result.clone());
        }

        let shape_ndim = self.arena[shape].ndim()?;
        let shape_manifold = &self.arena[shape].manifold;

        let is_shape_flat = shape_manifold.is_flat();
        let are_all_children_flat = self.arena[shape].boundary.iter().all(|boundary_elem| {
            let boundary_elem = &self.arena[boundary_elem.id];
            boundary_elem.manifold.is_flat() || boundary_elem.ndim().ok() == Some(0)
        });
        if is_shape_flat && are_all_children_flat {
            self.ensure_finite(shape)?;
            return self.flat_shape_simplices(shape);
        }

        // Start with a simplex blob that approximates the manifold (or the part
        // of it we care about, at least).
        let radius = self.radius_upper_bound(shape)?;
        let mut ret = if is_shape_flat {
            let center = shape_manifold.ipns().ipns_plane_pole();
            let tangent_space = shape_manifold.tangent_space()?;
            let basis_vectors = tangent_space.basis_at(cga::Point::Finite(center.clone()))?;
            let radius = radius * (shape_ndim as Float).sqrt();
            let resolution = 0;
            self.new_ball(center, &basis_vectors, radius, resolution)?
        } else {
            let center = shape_manifold.ipns().ipns_sphere_center();
            let tangent_space = Manifold::from_opns(
                shape_manifold.opns() ^ cga::Blade::NI,
                self.arena.space().ndim()?,
            )
            .context("unable to construct container space for hypersphere")?
            .tangent_space()?;
            let basis_vectors = tangent_space.basis_at(center.clone())?;
            let resolution = DEFAULT_SPHERE_RESOLUTION;
            self.new_sphere(&center.to_finite()?, &basis_vectors, radius, resolution)?
        };

        for boundary_elem in self.arena[shape].boundary.iter() {
            let cut = self.arena.signed_manifold_of_shape(boundary_elem)?;
            let max_simplex_edge_length = if cut.is_flat() || cut.ndim()? == 0 {
                None
            } else {
                Some(max_edge_length_for_radius(
                    cut.ipns()
                        .ipns_radius()
                        .context("unable to get radius of hypersphere")?,
                ))
            };
            let mut slice_op = SimplexSliceOp {
                manifold: shape_manifold,
                cut: &cut,
                max_simplex_edge_length,

                vertex_split_cache: AHashMap::new(),
                simplex_split_cache: AHashMap::new(),
                subdivision_cache: AHashMap::new(),
            };
            ret = self.cut_simplex_blob(&mut slice_op, &ret)?.inside();
        }

        self.shape_simplices_cache.insert(shape, ret.clone());

        Ok(ret)
    }

    fn flat_shape_simplices(&mut self, shape: ShapeId) -> Result<SimplexBlob> {
        // This alternative algorithm only works when the shape is flat convex
        // (which we can't guarantee if there's roundness involved anywhere) so
        // don't use it in general, but when we can use it it's much more
        // efficient.

        let shape_ndim = self.arena[shape].ndim()?;

        if shape_ndim == 1 {
            let boundary_elems = &self.arena[shape].boundary;
            ensure!(
                boundary_elems.len() == 1,
                "flat_shape_simplices algorithm \
                 only works on simple edges; got \
                 edge with complex boundary instead"
            );
            let boundary_elem = boundary_elems.iter().next().unwrap();
            let [a, b] = self.arena[boundary_elem.id].manifold.to_point_pair()?;
            Ok(Simplex::new([
                self.add_vertex(a.to_finite()?),
                self.add_vertex(b.to_finite()?),
            ])
            .into())
        } else {
            SimplexBlob::from_convex_hull(
                &self.arena[shape]
                    .boundary
                    .iter()
                    .map(|boundary_elem| self.shape_simplices(boundary_elem.id))
                    .collect::<Result<Vec<_>>>()?,
            )
        }
    }

    fn new_ball(
        &mut self,
        center: Vector,
        basis_vectors: &[Vector],
        radius: Float,
        resolution: u8,
    ) -> Result<SimplexBlob> {
        let mut ret = self.new_sphere(&center, basis_vectors, radius, resolution)?;

        // Take each simplex on the boundary of the ball and just add the center.
        let center = self.add_vertex(center);
        for simplex in &mut ret.0 {
            simplex.0.insert(center);
        }

        Ok(ret)
    }
    fn new_sphere(
        &mut self,
        center: &Vector,
        basis_vectors: &[Vector],
        radius: Float,
        resolution: u8,
    ) -> Result<SimplexBlob> {
        // Construct an octahedron.
        let octahedron_facets = SimplexBlob(
            basis_vectors
                .iter()
                .map(|basis_vector| basis_vector * radius)
                .map(|b| [center + &b, center - &b].map(|p| self.add_vertex(p)))
                .multi_cartesian_product()
                .map(|verts| Simplex(verts.into_iter().collect()))
                .collect(),
        );

        // Recursively subdivide until the edge length is short enough.
        let mut result = octahedron_facets;
        let m = Manifold::new_hypersphere(center, radius, self.arena.space().ndim()?);
        for _ in 0..resolution {
            let mut cache = Default::default();
            for facet in std::mem::take(&mut result).0 {
                result.extend(self.subdivide_simplex(&facet, &m, &mut cache)?);
            }
        }

        Ok(result)
    }

    fn subdivide_simplex(
        &mut self,
        s: &Simplex,
        m: &Manifold,
        cache: &mut AHashMap<Simplex, SimplexBlob>,
    ) -> Result<SimplexBlob> {
        if let Some(result) = cache.get(s) {
            return Ok(result.clone());
        }

        let result = if s.ndim()? < 1 {
            SimplexBlob::new([s.clone()])
        } else if let Some([a, b]) = s.try_into_array() {
            let midpoint = self.add_vertex(self.simplex_center(s, m)?);
            SimplexBlob::new([
                Simplex(Set64::from_iter([a, midpoint])),
                Simplex(Set64::from_iter([b, midpoint])),
            ])
        } else {
            // Split each facet.
            let mini_facet_blobs: Vec<SimplexBlob> = s
                .facets()?
                .map(|facet| self.subdivide_simplex(&facet, m, cache))
                .try_collect()?;

            let mut inner_facets = vec![];

            let mut tips: SmallVec<[Simplex; 6]> = smallvec![];
            // For each vertex in the original shape, construct one mini simplex
            // containing it.
            for v in s.0.iter() {
                // Find two mini facets containing the vertex; this will be
                // enough to get all of the vertices of the mini simplex formed
                // by them.
                let mini_simplex = Simplex(
                    mini_facet_blobs
                        .iter()
                        .flat_map(|mini_facet_blob| &mini_facet_blob.0)
                        .filter(|mini_facet| mini_facet.0.contains(&v))
                        .take(2)
                        .flat_map(|mini_facet| mini_facet.0.iter())
                        .collect(),
                );
                ensure!(mini_simplex.ndim()? == s.ndim()?);
                tips.push(mini_simplex.clone());

                // `mini_simplex` contains one final facet that isn't in
                // `mini_facet_blobs`.
                let mut inner_facet = mini_simplex;
                inner_facet.0.remove(&v);
                inner_facets.push(inner_facet);
            }

            // Remove the mini facets we've already used.
            let mut remaining_facet_blobs = mini_facet_blobs;
            for facet_blob in &mut remaining_facet_blobs {
                // A mini facet has already been used iff it contains one of the
                // original vertices.
                facet_blob
                    .0
                    .retain(|facet| s.0.iter().all(|v| !facet.0.contains(v)));
            }
            // Add the inner facets.
            remaining_facet_blobs.extend(inner_facets.into_iter().map(|f| SimplexBlob::new([f])));
            // Build new simplices using what's left.
            let mut result = if s.ndim()? == 2 {
                SimplexBlob::from_convex_hull(&remaining_facet_blobs)?
            } else {
                SimplexBlob::from_convex_hull_and_initial_vertex(
                    &remaining_facet_blobs,
                    self.add_vertex(self.simplex_center(s, m)?),
                )
            };

            // Don't forget the tips.
            result.0.extend(tips);

            result
        };

        cache.insert(s.clone(), result.clone());
        Ok(result)
    }

    fn add_vertex(&mut self, p: Vector) -> VertexId {
        *self.vertex_ids.entry(&p).or_insert_with(|| {
            let id = VertexId(self.vertices.len() as u32);
            self.vertices.push(p);
            id
        })
    }

    fn cut_simplex_blob(
        &mut self,
        op: &mut SimplexSliceOp<'_>,
        blob: &SimplexBlob,
    ) -> Result<SimplexSplit> {
        // Subdivide simplices until they are all less than the max edge length.
        let mut simplices_to_cut = blob.0.iter().cloned().collect_vec();
        if let Some(max_simplex_edge_length) = op.max_simplex_edge_length {
            let mut queue = std::mem::take(&mut simplices_to_cut);
            while let Some(s) = queue.pop() {
                if self.longest_edge_of_simplex(&s)? > max_simplex_edge_length
                    && self.simplex_intersects_cut(op, &s)?
                {
                    queue.extend(
                        self.subdivide_simplex(&s, op.manifold, &mut op.subdivision_cache)?
                            .0,
                    );
                } else {
                    simplices_to_cut.push(s);
                }
            }
        }

        // Cut the simplices.
        let mut ret = SimplexSplit::Flush;
        for s in simplices_to_cut {
            ret.extend(self.cut_simplex(op, &s)?);
        }
        Ok(ret)
    }

    fn simplex_intersects_cut(&self, _op: &mut SimplexSliceOp<'_>, _s: &Simplex) -> Result<bool> {
        // TODO: Actually compute whether the simplex intersects the cut. Always
        //       returning `true` produces very suboptimal meshes when
        //       small-radius spheres are involved.
        Ok(true)

        // // This function can't handle 0-dimensional cuts.
        // ensure!(op.cut.ndim()? > 0);

        // // If one vertex is on the inside and one is on the outside, then it
        // // definitely intersects.
        // let mut any_inside = false;
        // let mut any_outside = false;
        // for v in s.0.iter() {
        //     let which_side = op
        //         .cut
        //         .which_side_has_point(&self.vertex_point(v), op.manifold)?;
        //     any_inside |= which_side.is_any_inside;
        //     any_outside |= which_side.is_any_outside;
        //     if any_inside && any_outside {
        //         return Ok(true);
        //     }
        // }

        // // Early return `false` if any single facet excludes the cut.
        // for v in s.0.iter() {
        //     let mut facet = s.clone();
        //     facet.0.remove(&v);
        //     let facet_manifold = self.simplex_manifold(&facet);
        //     let which_side_has_point = op.cut.which_side_has_point(&facet, space);
        //     let which_side_has_manifold
        // }
    }

    fn cut_simplex(&mut self, op: &mut SimplexSliceOp<'_>, s: &Simplex) -> Result<SimplexSplit> {
        if let Some(result) = op.simplex_split_cache.get(s) {
            return Ok(result.clone());
        }

        ensure!(s.ndim()? >= 1);

        let result = if let Some([a, b]) = s.try_into_array() {
            self.cut_edge_uncached(op, [a, b])?
        } else {
            // Split each facet.
            let mut is_all_flush = true;
            let mut inside = vec![];
            let mut intersection = vec![];
            let mut intersection_facet = None;
            for facet in s.facets()? {
                match self.cut_simplex(op, &facet)? {
                    SimplexSplit::Flush => intersection_facet = Some(SimplexBlob::new([facet])),
                    SimplexSplit::NonFlush(result) => {
                        is_all_flush = false;
                        if !result.inside.0.is_empty() {
                            inside.push(result.inside);
                        }
                        if !result.intersection.0.is_empty() {
                            intersection.push(result.intersection);
                        }
                    }
                }
            }

            if is_all_flush {
                SimplexSplit::Flush
            } else if inside.is_empty() {
                SimplexSplit::NonFlush(NonFlushSimplexSplit::EMPTY)
            } else {
                println!();
                println!("context:");
                for (i, v) in self.vertices.iter().enumerate() {
                    println!("#{i} = {v}");
                }
                println!();
                println!();
                println!();
                println!("pre:  intersection = {}", intersection.iter().join(", "));
                if intersection_facet.is_some() {
                    println!(
                        "pre:  intersection_facet = {}",
                        intersection_facet.as_ref().unwrap()
                    );
                }
                let intersection = match intersection_facet {
                    Some(f) => f,
                    None => SimplexBlob::from_convex_hull(&intersection)?,
                };
                println!("inside = {}", inside.iter().join(", "));
                println!("intersection = {intersection}");
                inside.push(intersection.clone());
                let inside = SimplexBlob::from_convex_hull(&inside)?;
                SimplexSplit::NonFlush(NonFlushSimplexSplit {
                    inside,
                    intersection,
                })
            }
        };

        op.simplex_split_cache.insert(s.clone(), result.clone());
        Ok(result)
    }

    fn cut_edge_uncached(
        &mut self,
        op: &mut SimplexSliceOp<'_>,
        [a, b]: [VertexId; 2],
    ) -> Result<SimplexSplit> {
        // Sort the vertices into inside, outside, and intersection.
        let mut inside = Set64::new();
        let mut outside = Set64::new();
        let mut intersection = Set64::new();
        for v in [a, b] {
            match self.split_vertex(op, v)? {
                PointWhichSide::Inside => inside.insert(v),
                PointWhichSide::Outside => outside.insert(v),
                PointWhichSide::On => intersection.insert(v),
            };
        }

        println!(
            "cutting edge {a}..{b} ({},{},{})",
            inside.len(),
            outside.len(),
            intersection.len()
        );

        if inside.is_empty() && outside.is_empty() {
            ensure!(intersection.len() == 2);
            Ok(SimplexSplit::Flush)
        } else if inside.is_empty() {
            Ok(SimplexSplit::NonFlush(NonFlushSimplexSplit::EMPTY))
        } else {
            if !outside.is_empty() {
                // If one vertex is inside and one is outside, then create a new
                // vertex at the intersection point.
                let line = Manifold::new_line(
                    &self.vertex_point(a),
                    &self.vertex_point(b),
                    self.arena.space(),
                )?;
                let intersection_point = match line.split(&op.cut, self.arena.space())? {
                    ManifoldSplit::Split {
                        intersection_manifold,
                    } => {
                        let [c, d] = intersection_manifold
                            .opns()
                            .point_pair_to_points()
                            .context("intersection is not point pair")?
                            .map(|p| p.to_finite().ok());
                        // Select the intersection point that is within the line
                        // segment by its comparing distance to each endpoint.
                        crate::util::merge_options(c, d, |c, d| {
                            std::cmp::min_by_key(c, d, |p| {
                                let p_distance = |v| FloatOrd((p - &self[v]).mag2());
                                std::cmp::max(p_distance(a), p_distance(b))
                            })
                        })
                        .context("no finite intersection")?
                    }

                    _ => bail!(
                        "found a counterexample to the \
                         intermediate value theorem",
                    ),
                };
                intersection.insert(self.add_vertex(intersection_point));
            }

            inside.extend(intersection.iter());
            ensure!(inside.len() == 2);
            ensure!(intersection.len() <= 1);
            Ok(SimplexSplit::NonFlush(NonFlushSimplexSplit {
                inside: Simplex(inside).into(),
                intersection: if intersection.is_empty() {
                    SimplexBlob::EMPTY
                } else {
                    Simplex(intersection).into()
                },
            }))
        }
    }

    fn split_vertex(&mut self, op: &mut SimplexSliceOp<'_>, v: VertexId) -> Result<PointWhichSide> {
        if let Some(result) = op.vertex_split_cache.get(&v) {
            return Ok(result.clone());
        }

        let point = self.vertex_point(v);
        let result = op.cut.which_side_has_point(&point, self.arena.space())?;

        op.vertex_split_cache.insert(v, result.clone());
        Ok(result)
    }

    fn longest_edge_of_simplex(&self, s: &Simplex) -> Result<Float> {
        Ok(s.edges()
            .map(|[v1, v2]| FloatOrd((&self[v1] - &self[v2]).mag2()))
            .max()
            .context("simplex has no edges")?
            .0
            .sqrt())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct Simplex(Set64<VertexId>);
impl fmt::Display for Simplex {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Simplex({})", self.0.iter().join(", "))
    }
}
impl Simplex {
    fn new(verts: impl IntoIterator<Item = VertexId>) -> Self {
        Simplex(verts.into_iter().collect())
    }
    fn ndim(&self) -> Result<u8> {
        (self.0.len() as u8)
            .checked_sub(1)
            .context("simplex cannot be empty")
    }
    fn try_into_array<const N: usize>(&self) -> Option<[VertexId; N]> {
        (self.0.len() == N).then(|| {
            let mut it = self.0.iter();
            [(); N].map(|_| it.next().unwrap())
        })
    }
    /// Returns all 1-dimensional elemenst of the simplex.
    fn edges(&self) -> impl '_ + Iterator<Item = [VertexId; 2]> {
        let verts: SmallVec<[VertexId; 8]> = self.0.iter().collect();
        verts
            .into_iter()
            .tuple_combinations()
            .map(|(v1, v2)| [v1, v2])
    }
    /// Returns all (N-1)-dimensional elements of the simplex.
    fn facets(&self) -> Result<impl '_ + Iterator<Item = Simplex>> {
        let ndim = self.ndim()?;
        let facet_ndim = ndim.checked_sub(1).context("0D simplex has no facets")?;
        Ok(self.elements(facet_ndim))
    }
    /// Returns all elements of the simplex with a given number of dimensions.
    fn elements(&self, ndim: u8) -> impl '_ + Iterator<Item = Simplex> {
        self.0
            .iter()
            .combinations(ndim as usize + 1)
            .map(|verts| Simplex(Set64::from_iter(verts)))
    }
}

/// Convex polytope made of simplices.
#[derive(Debug, Default, Clone)]
struct SimplexBlob(SmallVec<[Simplex; 2]>);
impl fmt::Display for SimplexBlob {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Blob[{}]", self.0.iter().join(", "))
    }
}
impl From<Simplex> for SimplexBlob {
    fn from(s: Simplex) -> Self {
        SimplexBlob::new([s])
    }
}
impl SimplexBlob {
    const EMPTY: Self = SimplexBlob(SmallVec::new_const());

    fn new(simplices: impl IntoIterator<Item = Simplex>) -> Self {
        SimplexBlob(simplices.into_iter().collect())
    }

    fn from_convex_hull(facets: &[SimplexBlob]) -> Result<Self> {
        let Some(arbitrary_facet) = facets.iter().find_map(|f| f.0.get(0)) else {
            return Ok(SimplexBlob::EMPTY);
        };
        let facet_ndim = arbitrary_facet.ndim()?;

        ensure!(
            facets
                .iter()
                .flat_map(|f| &f.0)
                .all(|s| s.ndim().ok() == Some(facet_ndim)),
            "cannot construct simplex blob from \
             dimension-mismatched convex hull",
        );

        let facet_simplices = facets.iter().flat_map(|f| &f.0);
        let vertex_set: Set64<VertexId> = facet_simplices.flat_map(|s| s.0.iter()).collect();

        // Optimization: if the number of simplices equals the facet dimension
        // plus 2 equals the nubmer of vertices, then the result is a single
        // simplex.
        let number_of_simplices = facets.iter().map(|f| f.0.len()).sum::<usize>();
        let is_single_simplex = number_of_simplices == facet_ndim as usize + 2
            && number_of_simplices == vertex_set.len();
        if is_single_simplex {
            // Construct the single simplex.
            Ok(SimplexBlob::new([Simplex(vertex_set)]))
        } else {
            // Pick a vertex to start from. This `.unwrap()` always succeeds
            // because `.ndim()` succeded.
            let initial_vertex = arbitrary_facet.0.iter().next().unwrap();
            Ok(SimplexBlob::from_convex_hull_and_initial_vertex(
                facets,
                initial_vertex,
            ))
        }
    }

    fn from_convex_hull_and_initial_vertex(
        facets: &[SimplexBlob],
        initial_vertex: VertexId,
    ) -> Self {
        let mut ret = smallvec![];

        // For every facet that does not contain that vertex ...
        for facet in facets {
            if facet.0.iter().all(|s| !s.0.contains(&initial_vertex)) {
                // ... for every simplex in that facet ...
                for simplex in &facet.0 {
                    // ... construct a new simplex that will be in the result.
                    let mut simplex = simplex.clone();
                    simplex.0.insert(initial_vertex);
                    // And add that simplex, if it's not a duplicate.
                    if !ret.contains(&simplex) {
                        ret.push(simplex);
                    }
                }
            }
        }

        SimplexBlob(ret)
    }

    fn extend(&mut self, other: SimplexBlob) {
        self.0.extend(other.0);
    }
}

/// Result of splitting a simplex by a manifold.
#[derive(Debug, Clone)]
enum SimplexSplit {
    Flush,
    NonFlush(NonFlushSimplexSplit),
}
impl SimplexSplit {
    fn extend(&mut self, other: SimplexSplit) {
        match (self, other) {
            (_, SimplexSplit::Flush) => (),
            (a @ SimplexSplit::Flush, b) => *a = b,
            (SimplexSplit::NonFlush(a), SimplexSplit::NonFlush(b)) => {
                a.inside.extend(b.inside);
                a.intersection.extend(b.intersection);
            }
        }
    }

    fn inside(self) -> SimplexBlob {
        match self {
            SimplexSplit::Flush => SimplexBlob::EMPTY,
            SimplexSplit::NonFlush(result) => result.inside,
        }
    }
}

#[derive(Debug, Clone)]
struct NonFlushSimplexSplit {
    /// N-dimensional simplices inside the cut.
    inside: SimplexBlob,
    /// (N-1)-dimensional simplices flush with the cut.
    intersection: SimplexBlob,
}
impl NonFlushSimplexSplit {
    const EMPTY: Self = NonFlushSimplexSplit {
        inside: SimplexBlob::EMPTY,
        intersection: SimplexBlob::EMPTY,
    };
}

#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct VertexId(u32);
impl fmt::Display for VertexId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "#{}", self.0)
    }
}
impl tinyset::Fits64 for VertexId {
    unsafe fn from_u64(x: u64) -> Self {
        Self(x as u32)
    }

    fn to_u64(self) -> u64 {
        self.0 as u64
    }
}

struct SimplexSliceOp<'a> {
    /// Manifold that the simplices are trying to approximate.
    manifold: &'a Manifold,
    /// Manifold by which to cut the simplices.
    cut: &'a Manifold,
    /// Maximum edge length for a simplex before being cut, or `None` if there
    /// is no maximum.
    max_simplex_edge_length: Option<Float>,

    /// Cached results of splitting vertices.
    vertex_split_cache: AHashMap<VertexId, PointWhichSide>,
    /// Cached results of splitting simplices.
    simplex_split_cache: AHashMap<Simplex, SimplexSplit>,
    /// Cached results of subdividing simplices.
    subdivision_cache: AHashMap<Simplex, SimplexBlob>,
}

/// Centroid and Lebasgue measure of a polytope. In simpler terms: the "center
/// of mass" and "N-dimensional mass" of a polytope.
#[derive(Debug, Clone, PartialEq)]
pub struct Centroid {
    /// Lebasgue measure (https://w.wiki/FLd) as a blade in Euclidean geometric
    /// algebra (no e₋ or e₊ components).
    pub blade: cga::Blade,
    /// Center of mass.
    pub com: Vector,
}
impl fmt::Display for Centroid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Centroid {{ blade = {}, com = {} }}",
            self.blade, self.com,
        )
    }
}
impl Default for Centroid {
    fn default() -> Self {
        Self {
            blade: cga::Blade::ZERO,
            com: Vector::EMPTY,
        }
    }
}
impl Sum<Centroid> for Centroid {
    fn sum<I: Iterator<Item = Centroid>>(iter: I) -> Self {
        // This function assumes that all the masses are in the same subspaace.

        let mut iter = iter.filter(|centroid| !centroid.blade.is_zero()).peekable();
        let Some(first) = iter.peek() else {
            return Centroid::default();
        };

        // Some of these masses may have opposite signs. We want all masses to
        // be positive, so pick some component to normalize the signs with
        // respect to.
        let normalization_term = first.blade.mv().most_significant_term();
        let unit_mass = &first.blade * normalization_term.coef.recip();

        let mut total_com = Vector::EMPTY;
        let mut total_weight = 0.0;

        for it in iter {
            let weight = it.blade.mv()[normalization_term.axes].abs();
            total_com += it.com * weight;
            total_weight += weight;
        }

        Centroid {
            blade: unit_mass * total_weight,
            com: total_com / total_weight,
        }
    }
}

/// Returns the required resolution of a simplex blob for a given radius of
/// sphere that may intersect it. For example, if
/// `max_edge_length_for_radius(3.0)` returns `0.75`, then simplices must be
/// subdivided until their edges are at most 0.75 units long before they can be
/// intersected by a sphere with radius 3.0.
fn max_edge_length_for_radius(r: Float) -> Float {
    // TODO: Make these magic constants configurable.
    (r / 4.0).clamp(0.01, 0.5) + EPSILON
}

#[cfg(test)]
mod tests {
    use super::*;

    /// This test just ensures that simplexification of a cube doesn't panic.
    /// Run it with `-- --nocapture` to see the output.
    #[test]
    fn test_cube_simplexification() {
        const NDIM: u8 = 3;

        // Construct a 2^NDIM cube.
        let mut arena = ShapeArena::new_euclidean_cga(NDIM);
        for ax in 0..NDIM {
            let v = Vector::unit(ax);
            for sign in [Sign::Neg, Sign::Pos] {
                arena.carve_plane(&v * sign.to_float(), 1.0, 0).unwrap();
            }
            arena.slice_plane(v, 0.0).unwrap();
        }
        println!("{arena}");

        let mut simplexifier = Simplexifier::new(&arena);
        for &shape in arena.roots() {
            println!("simplexifying {shape}");
            let simplices = simplexifier.shape_simplices(shape).unwrap();
            println!("... simplices: {simplices}",);
            let centroid = simplexifier.shape_centroid(shape).unwrap();
            println!("... centroid: {centroid}",);
            println!();

            assert!(!simplices.0.is_empty());
        }
        println!("Vertex positions:");
        for (i, v) in simplexifier.vertices.iter().enumerate() {
            println!("#{i} = {v}");
        }
    }
}
