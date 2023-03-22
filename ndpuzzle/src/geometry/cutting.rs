//! Algorithm for slicing space into pieces based on Andrey Astrelin's
//! implementation of `GenCube()` in Magic Puzzle Ultimate (FaceCuts.cs).

// TODO potential optimizations in this file:
// - memoize manifolds and cache each manifold's intersection with the cut
// - cache each shape's closest point in either direction of the cut

use ahash::AHashMap;
use anyhow::{bail, ensure, Context, Result};
use slab::Slab;
use std::fmt;
use std::ops::{Index, Neg};
use tinyset::Set64;

use super::manifold::*;
use super::shape::*;
use crate::math::{approx_eq, Sign};

/// Set of shapes in a space.
#[derive(Debug, Clone)]
pub struct ShapeArena<M> {
    /// Space that all shapes inhabit.
    space: M,
    /// All shapes and elements of them.
    shapes: Slab<Shape<M>>,
    /// Top-level "root" shapes.
    roots: Vec<ShapeId>,
}

impl<M> Index<ShapeId> for ShapeArena<M> {
    type Output = Shape<M>;

    fn index(&self, index: ShapeId) -> &Self::Output {
        &self.shapes[index.0 as usize]
    }
}

impl<M: Manifold> fmt::Display for ShapeArena<M> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Shape arena in space {}", self.space)?;
        for &root_id in &self.roots {
            self.display_shape(f, ShapeRef::from(root_id), Sign::Pos, 1)?;
        }
        Ok(())
    }
}

impl<M: Manifold> ShapeArena<M> {
    /// Constructs a new shape arena containing only a shape representing the
    /// whole space.
    pub fn new(space: M) -> Self {
        let mut shapes = Slab::new();
        let id = shapes.insert(Shape::whole_space(space.clone()));
        let roots = vec![ShapeId(id as u32)];

        Self {
            space,
            shapes,
            roots,
        }
    }

    /// Returns the manifold representing the whole space.
    pub fn space(&self) -> &M {
        &self.space
    }
    /// Returns the list of root shapes, in canonical order based on the cuts.
    pub fn roots(&self) -> &[ShapeId] {
        &self.roots
    }
    /// Returns whether the shape arena is empty (contains no root shapes).
    pub fn is_empty(&self) -> bool {
        self.roots.is_empty()
    }

    /// Adds a shape.
    fn add(&mut self, shape: Shape<M>) -> Result<ShapeRef> {
        // Check the rank of each boundary shape.
        let ndim = shape.manifold.ndim()?;
        for boundary_shape in shape.boundary.iter() {
            ensure!(ndim == self[boundary_shape.id].ndim()? + 1);
        }

        let idx = self.shapes.insert(shape);
        Ok(ShapeRef {
            id: ShapeId(idx as _),
            sign: Sign::Pos,
        })
    }
    /// Adds a shape using the manifold of an existing shape, or reuses the
    /// existing shape if possible.
    fn add_subshape(
        &mut self,
        old_shape: ShapeId,
        new_boundary: Set64<ShapeRef>,
    ) -> Result<ShapeRef> {
        if new_boundary == self[old_shape].boundary {
            Ok(ShapeRef::from(old_shape))
        } else {
            self.add(Shape {
                manifold: self[old_shape].manifold.clone(),
                boundary: new_boundary,
            })
        }
    }

    /// Returns a shape's manifold, flipped depending on the sign of the
    /// reference to the shape.
    pub fn signed_mainfold_of_shape(&self, shape: ShapeRef) -> Result<M> {
        let m = &self[shape.id].manifold;
        match shape.sign {
            Sign::Pos => Ok(m.clone()),
            Sign::Neg => m.flip(),
        }
    }
    /// Returns the sign difference between two manifolds, or `None` if they are
    /// different.
    fn sign_difference(
        &self,
        a: impl SignedManifold<M>,
        b: impl SignedManifold<M>,
    ) -> Result<Option<Sign>> {
        match a
            .get_manifold_from(self)?
            .relative_orientation(b.get_manifold_from(self)?)
        {
            Some(sign) => Ok(Some(sign * a.sign() * b.sign())),
            None => Ok(None),
        }
    }

    /// Garbage-collects unused shapes.
    pub fn gc(&mut self) {
        let total = self.shapes.len();
        let mut keys_to_delete = self
            .shapes
            .iter()
            .map(|(i, _shape)| ShapeId(i as u32))
            .collect();
        for &root in &self.roots {
            self.gc_mark_recursive(&mut keys_to_delete, root);
        }
        let num_deleted = keys_to_delete.len();
        for id in keys_to_delete.iter() {
            self.shapes.remove(id.0 as usize);
        }
        let percent = num_deleted * 100 / total;
        log::trace!("Garbage-collected {num_deleted}/{total} shapes ({percent}%)");
    }
    fn gc_mark_recursive(&self, keys_to_delete: &mut Set64<ShapeId>, shape_to_keep: ShapeId) {
        if keys_to_delete.remove(&shape_to_keep) {
            for child in self[shape_to_keep].boundary.iter() {
                self.gc_mark_recursive(keys_to_delete, child.id);
            }
        }
    }

    /// Cuts all shapes in the arena.
    pub fn cut(&mut self, params: CutParams<M>) -> Result<()> {
        log::trace!("Cutting all shapes by {}", params.cut);
        let f = |remove| if remove { "DELETE" } else { "KEEP" };
        log::trace!("  ... inside: {}", f(params.remove_inside));
        log::trace!("  ... outside: {}", f(params.remove_outside));

        let mut op = SliceOperation::new(params.cut);

        for root_id in std::mem::take(&mut self.roots) {
            match self
                .cut_shape(ShapeRef::from(root_id), &mut op)
                .context("error cutting shape")?
            {
                SplitResult::Flush => bail!("root shape is flush with cut"),
                SplitResult::NonFlush {
                    inside,
                    outside,
                    intersection_shape: _,
                } => {
                    if !params.remove_inside {
                        self.roots.extend(inside.map(|s| s.id));
                    }
                    if !params.remove_outside {
                        self.roots.extend(outside.map(|s| s.id));
                    }
                }
            }
        }

        self.gc();

        Ok(())
    }

    /// Cuts a shape.
    fn cut_shape(
        &mut self,
        shape: ShapeRef,
        slice_op: &mut SliceOperation<M>,
    ) -> Result<SplitResult> {
        let result = match slice_op.results_cache.get(&shape.id) {
            Some(result) => result.clone(),
            None => {
                let result = self
                    .cut_shape_uncached(shape.id, slice_op)
                    .with_context(|| format!("error cutting shape {shape}"))?;
                slice_op.results_cache.insert(shape.id, result.clone());
                result
            }
        };

        Ok(match shape.sign {
            Sign::Pos => result,
            Sign::Neg => -result,
        })
    }
    /// Cuts a shape without caching the result.
    fn cut_shape_uncached(
        &mut self,
        shape: ShapeId,
        slice_op: &mut SliceOperation<M>,
    ) -> Result<SplitResult> {
        match self[shape]
            .manifold
            .split(&slice_op.divider, &self.space)
            .context("error splitting manifold")?
        {
            ManifoldSplit::Flush(_) => Ok(SplitResult::Flush),
            ManifoldSplit::Inside => Ok(SplitResult::all_inside(shape)),
            ManifoldSplit::Outside => Ok(SplitResult::all_outside(shape)),

            ManifoldSplit::Split {
                intersection_manifold,
            } if self[shape].ndim()? == 1 => {
                let intersection_shape = self.add(Shape::whole_space(intersection_manifold))?;

                let inside_boundary = self
                    .incremental_simplify_intervals_intersection(
                        &self[shape].boundary.clone(),
                        intersection_shape,
                        &self[shape].manifold.clone(),
                    )
                    .context("error simplifying 1D boundary of inside")?;
                let inside = if let Some(boundary) = inside_boundary {
                    Some(self.add_subshape(shape, boundary)?)
                } else {
                    None
                };

                let outside_boundary = self
                    .incremental_simplify_intervals_intersection(
                        &self[shape].boundary.clone(),
                        -intersection_shape,
                        &self[shape].manifold.clone(),
                    )
                    .context("error simplifying 1D boundary of outside")?;
                let outside = if let Some(boundary) = outside_boundary {
                    Some(self.add_subshape(shape, boundary)?)
                } else {
                    None
                };

                Ok(SplitResult::NonFlush {
                    inside,
                    outside,
                    intersection_shape: Some(intersection_shape),
                })
            }

            ManifoldSplit::Split {
                intersection_manifold,
            } => {
                // (N-1)-dimensional shapes that together comprise the boundary
                // of `shape ∩ cut`
                let mut self_boundary_of_inside = Set64::new();
                // (N-1)-dimensional shapes that together comprise the boundary
                // of `shape ∩ ~cut`
                let mut self_boundary_of_outside = Set64::new();
                // (N-2)-dimensional shapes that together comprise the boundary
                // of `shape ∩ boundary(cut)`
                let mut intersection_boundary = Set64::new();
                // (N-1)-dimensional shape that is `shape ∩ boundary(cut)`
                let mut intersection_shape = None;

                // Split each of the "child" shapes that comprise the boundary
                // of `shape`.
                for child in self[shape].boundary.clone() {
                    match self.cut_shape(child, slice_op)? {
                        SplitResult::Flush => {
                            ensure!(intersection_shape.is_none(), "multiple intersection shapes");
                            intersection_shape = Some(child);
                        }
                        SplitResult::NonFlush {
                            inside,
                            outside,
                            intersection_shape,
                        } => {
                            self_boundary_of_inside.extend(inside);
                            self_boundary_of_outside.extend(outside);
                            intersection_boundary.extend(intersection_shape.map(|s| -s));
                        }
                    }
                }

                let mut any_inside = true;
                let mut any_outside = true;

                // Is `shape ∩ boundary(cut)` (`intersection_shape`) nonempty?
                //
                // If `intersection_shape` already exists, it is obviously
                // nonempty.
                //
                // The second condition checks whether `boundary(shape ∩
                // boundary(cut))` is nonempty. If `shape ∩ boundary(cut)` has
                // no boundary, then it is either empty or the whole manifold.
                //
                // The third condition checks whether it is the whole manifold;
                // i.e., whether `shape ∩ boundary(cut) ⊆ shape`.
                //
                // Thus if none of these conditions are met, then `shape ∩
                // boundary(cut)` is empty.
                let is_intersection_nonempty = intersection_shape.is_some()
                    || !intersection_boundary.is_empty()
                    || self.shape_completely_contains_manifold(shape, &intersection_manifold)?;

                if let Some(shape) = intersection_shape {
                    // There already exists an intersection shape, so either
                    // `shape ∩ cut` is empty or `shape ∩ ~cut` is empty.
                    let sign = self
                        .sign_difference(shape, &intersection_manifold)?
                        .context(
                            "manifold of intersection shape does \
                             not match intersection manifold",
                        )?;
                    match sign {
                        Sign::Pos => any_outside = false,
                        Sign::Neg => any_inside = false,
                    }
                } else if is_intersection_nonempty {
                    // Construct the shape that is `shape ∩ boundary(cut)`.
                    if intersection_manifold.ndim()? == 1 {
                        let simplified_intersection_boundary = self
                            .simplify_intervals_intersection(
                                intersection_boundary.iter(),
                                &intersection_manifold,
                            )
                            .context("error simplifying 1D intersection boundary")?;
                        if let Some(boundary) = simplified_intersection_boundary {
                            intersection_shape = Some(self.add(Shape {
                                manifold: intersection_manifold,
                                boundary,
                            })?);
                        } else {
                            // `shape ∩ boundary(cut)` is empty!
                            if self_boundary_of_inside.is_empty() {
                                // There's nothing to bound `shape ∩ cut`, so it
                                // must be empty.
                                any_inside = false;
                            }
                            if self_boundary_of_outside.is_empty() {
                                // There's nothing to bound `shape ∩ ~cut`, so
                                // it must be empty.
                                any_outside = false;
                            }
                        }
                    } else {
                        intersection_shape = Some(self.add(Shape {
                            manifold: intersection_manifold,
                            boundary: intersection_boundary,
                        })?);
                    }
                }

                // Is `shape ∩ boundary(cut)` nonempty?
                if let Some(common_boundary) = intersection_shape {
                    // `shape ∩ boundary(cut)` is part of the boundary of `shape
                    // ∩ cut` and part of the boundary of `shape ∩ ~cut`.
                    self_boundary_of_inside.insert(common_boundary);
                    self_boundary_of_outside.insert(-common_boundary);
                }

                // At this point, we have finished computing `boundary(shape ∩
                // cut)` and `boundary(shape ∩ !cut)`. `shape.manifold` is split
                // by the cut, so if `boundary(shape ∩ cut)` is empty then
                // `shape ∩ cut` is empty.
                any_inside &= !self_boundary_of_inside.is_empty();
                // And ditto for `shape ∩ ~cut`.
                any_outside &= !self_boundary_of_outside.is_empty();

                // Construct the N-dimensional shape that is `self ∩ cut`
                let inside = if any_inside {
                    Some(self.add_subshape(shape, self_boundary_of_inside)?)
                } else {
                    None
                };
                // Construct the N-dimensional shape that is `self ∩ ~cut`
                let outside = if any_outside {
                    Some(self.add_subshape(shape, self_boundary_of_outside)?)
                } else {
                    None
                };

                Ok(SplitResult::NonFlush {
                    inside,
                    outside,
                    intersection_shape,
                })
            }
        }
    }

    /// Returns whether `manifold` (which is assumed to be flush with the
    /// manifold of `shape`) is completely inside `shape`.
    fn shape_completely_contains_manifold(&self, shape: ShapeId, manifold: &M) -> Result<bool> {
        let shape_manifold = &self[shape].manifold;
        for boundary_elem in self[shape].boundary.iter() {
            let boundary_elem_manifold = &self[boundary_elem.id].manifold;
            let which_side =
                manifold.which_side(boundary_elem_manifold, shape_manifold)? * boundary_elem.sign;
            if which_side.is_any_outside {
                return Ok(false);
            }
        }
        Ok(true)
    }

    /// Simplifies a subset of a 1D manifold represented as the intersection of
    /// a set of intervals, where each interval is represented as a point pair.
    fn simplify_intervals_intersection(
        &mut self,
        intervals: impl IntoIterator<Item = ShapeRef>,
        space: &M,
    ) -> Result<Option<Set64<ShapeRef>>> {
        let mut simplified = Set64::new();
        for new_elem in intervals {
            match self.incremental_simplify_intervals_intersection(&simplified, new_elem, space)? {
                Some(b) => simplified = b,
                None => return Ok(None),
            }
        }
        Ok(Some(simplified))
    }
    /// Intersects a set of intervals with another interval, where each interval
    /// is represented as a point pair.
    fn incremental_simplify_intervals_intersection(
        &mut self,
        existing_intervals: &Set64<ShapeRef>,
        mut new_interval: ShapeRef,
        space: &M,
    ) -> Result<Option<Set64<ShapeRef>>> {
        let [a, b] = self.shape_to_point_pair(new_interval)?;
        if approx_eq(&a, &b) {
            // The new interval contains the whole space and so has no effect.
            return Ok(Some(existing_intervals.clone()));
        }

        let mut simplified = Set64::new();
        for existing_interval in existing_intervals.iter() {
            // The intersection of intervals is the complement of the union of
            // the complements. (Negating a point pair corresponds to taking the
            // complement of an interval.)
            match self.try_merge_intervals(-existing_interval, -new_interval, space)? {
                MergedInterval::Old(shape) => new_interval = -shape,
                MergedInterval::New(manifold) => {
                    new_interval = self.add(Shape::whole_space(manifold.flip()?))?;
                }

                MergedInterval::WholeSpace => return Ok(None), // whole space is excluded; there's nothing left

                MergedInterval::NoIntersection => {
                    simplified.insert(existing_interval);
                }
            }
        }
        simplified.insert(new_interval);
        Ok(Some(simplified))
    }
    /// If two intervals (including their boundaries) intersect at all, retuns
    /// the combined interval. Otherwise returns `None`.
    fn try_merge_intervals(
        &self,
        interval1: ShapeRef,
        interval2: ShapeRef,
        space: &M,
    ) -> Result<MergedInterval<M>> {
        let ab = interval1;
        let pq = interval2;
        let [a, b] = self.shape_to_point_pair(ab)?;
        let [p, q] = self.shape_to_point_pair(pq)?;

        if approx_eq(&a, &p) && approx_eq(&b, &q) {
            // The intervals are the same.
            return Ok(MergedInterval::Old(ab));
        }

        let ab_has_p = self.closed_interval_contains_point(ab, &p, space)?;
        let ab_has_q = self.closed_interval_contains_point(ab, &q, space)?;
        let pq_has_a = self.closed_interval_contains_point(pq, &a, space)?;
        let pq_has_b = self.closed_interval_contains_point(pq, &b, space)?;
        let ab_has_pq = ab_has_p && ab_has_q;
        let pq_has_ab = pq_has_a && pq_has_b;

        if ab_has_pq && pq_has_ab {
            return Ok(MergedInterval::WholeSpace);
        }

        if ab_has_pq {
            return Ok(MergedInterval::Old(ab));
        }

        if pq_has_ab {
            return Ok(MergedInterval::Old(pq));
        }

        let start = if ab_has_p {
            a
        } else if pq_has_a {
            p
        } else {
            return Ok(MergedInterval::NoIntersection);
        };

        let end = if ab_has_q {
            b
        } else if pq_has_b {
            q
        } else {
            return Ok(MergedInterval::NoIntersection);
        };

        Ok(MergedInterval::New(M::new_point_pair(&start, &end, space)?))
    }
    /// Returns whether an interval (**including** its boundary) contains a
    /// point.
    fn closed_interval_contains_point(
        &self,
        interval: impl SignedManifold<M>,
        point: &M::Point,
        space: &M,
    ) -> Result<bool> {
        let interval_manifold = interval.get_manifold_from(self)?;
        let which_side = interval_manifold.which_side_has_point(point, space)? * interval.sign();
        Ok(!which_side.is_any_outside)
    }
    /// Returns the pair of points represented by a 0D manifold.
    fn shape_to_point_pair(&self, shape: impl SignedManifold<M>) -> Result<[M::Point; 2]> {
        let [a, b] = shape.get_manifold_from(self)?.to_point_pair()?;
        match shape.sign() {
            Sign::Pos => Ok([a, b]),
            Sign::Neg => Ok([b, a]),
        }
    }

    fn display_shape(
        &self,
        f: &mut fmt::Formatter<'_>,
        shape: ShapeRef,
        sign: Sign,
        indent: u8,
    ) -> fmt::Result {
        for _ in 0..indent {
            write!(f, "  ")?;
        }
        write!(f, "{}#{:<5}", shape.sign, shape.id.0)?;
        if let Ok(m) = self.signed_mainfold_of_shape(shape) {
            writeln!(f, "{m}")?;
        }
        for child in self[shape.id].boundary.iter() {
            self.display_shape(f, child, shape.sign * sign, indent + 1)?;
        }
        Ok(())
    }
}

/// Parameters for cutting a bunch of shapes.
#[derive(Debug, Clone)]
pub struct CutParams<M> {
    /// Closed, oriented manifold along which to cut.
    pub cut: M,
    /// Whether to remove the shapes on the "inside" of the cut.
    pub remove_inside: bool,
    /// Whether to remove the shapes on the "outside" of the cut.
    pub remove_outside: bool,
}

/// In-progress slicing operation.
#[derive(Debug)]
struct SliceOperation<M> {
    /// Closed, oriented manifold that divides the entire space into "inside"
    /// and "outside."
    divider: M,
    /// Cache of the result of splitting individual shapes.
    results_cache: AHashMap<ShapeId, SplitResult>,
}
impl<M> SliceOperation<M> {
    fn new(divider: M) -> Self {
        Self {
            divider,
            results_cache: AHashMap::new(),
        }
    }
}

/// Result of splitting an N-dimensional object by a manifold.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum SplitResult<R = ShapeRef> {
    /// The whole object is flush with the slice.
    Flush,
    /// Some part of the object is not flush with the slice.
    NonFlush {
        /// N-dimensional portion of the object that is inside the slice, if
        /// any. This may be the whole object.
        inside: Option<R>,
        /// N-dimensional portion of the object that is outside the slice, if
        /// any. This may be the whole object.
        outside: Option<R>,

        /// (N-1)-dimensional intersection of the object with the slicing
        /// manifold, if any.
        intersection_shape: Option<R>,
    },
}
impl Neg for SplitResult<ShapeRef> {
    type Output = Self;

    fn neg(mut self) -> Self::Output {
        fn negate_option_shape_ref(r: &mut Option<ShapeRef>) {
            if let Some(r) = r {
                *r = -*r
            }
        }

        if let SplitResult::NonFlush {
            inside,
            outside,
            intersection_shape,
        } = &mut self
        {
            negate_option_shape_ref(inside);
            negate_option_shape_ref(outside);
            negate_option_shape_ref(intersection_shape);
        }

        self
    }
}

impl SplitResult<ShapeRef> {
    fn all_inside(shape: impl Into<ShapeRef>) -> Self {
        Self::NonFlush {
            inside: Some(shape.into()),
            outside: None,
            intersection_shape: None,
        }
    }
    fn all_outside(shape: impl Into<ShapeRef>) -> Self {
        Self::NonFlush {
            inside: None,
            outside: Some(shape.into()),
            intersection_shape: None,
        }
    }
}

trait SignedManifold<M> {
    fn get_manifold_from<'a>(&'a self, arena: &'a ShapeArena<M>) -> Result<&'a M>;
    fn sign(&self) -> Sign;
}
impl<M: Manifold> SignedManifold<M> for ShapeRef {
    fn get_manifold_from<'a>(&'a self, arena: &'a ShapeArena<M>) -> Result<&'a M> {
        Ok(&arena[self.id].manifold)
    }
    fn sign(&self) -> Sign {
        self.sign
    }
}
impl<M: Manifold> SignedManifold<M> for M {
    fn get_manifold_from<'a>(&'a self, _arena: &'a ShapeArena<M>) -> Result<&'a M> {
        Ok(self)
    }
    fn sign(&self) -> Sign {
        Sign::Pos
    }
}
impl<M: Manifold> SignedManifold<M> for &M {
    fn get_manifold_from<'a>(&'a self, _arena: &'a ShapeArena<M>) -> Result<&'a M> {
        Ok(self)
    }
    fn sign(&self) -> Sign {
        Sign::Pos
    }
}

enum MergedInterval<M> {
    Old(ShapeRef),
    New(M),
    WholeSpace,
    NoIntersection,
}
