//! Infinite Euclidean space in which shapes can be constructed.
//!
//! In this module:
//! - A 0-dimensional manifold is always a pair of points.
//! - An N-dimensional manifold where N>0 is always closed (compact and with no
//!   boundary). More specifically, it is a hyperplane or hypersphere,
//!   represented using an OPNS blade in the conformal geometric algebra.
//! - The **inside** and **outside** of a manifold are the half-spaces enclosed
//!   by it when embedded with an orientation into another manifold with one
//!   more dimension. In conformal geometry, the inside and outside must be
//!   determined by the orientation of the manifold rather than which half-space
//!   is finite.
//! - An N-dimensional **shape** is the intersection of the **inside**s of
//!   finitely many (N-1)-dimensional manifolds; equivalently, it is the
//!   intersection of finitely many (N-1)-dimensional shapes.

use std::cmp::Ordering;
use std::collections::{hash_map, HashMap};
use std::fmt;
use std::ops::{BitOr, Index, Mul, MulAssign, Neg};

use anyhow::{anyhow, bail, ensure, Context, Result};
use float_ord::FloatOrd;
use hypermath::collections::ApproxHashMap;
use hypermath::prelude::*;
use itertools::Itertools;
use slab::Slab;
use tinyset::Set64;

mod cut;
mod manifold;
mod results;
mod shape;
mod shapeset;
mod signedref;

use cut::CutOp;
pub use cut::{CutInProgress, CutParams, ShapeFate};
pub use manifold::ManifoldData;
use results::{ManifoldWhichSide, MergedInterval, ShapeSplitResult};
pub use shape::ShapeData;
pub use shapeset::ShapeSet;
pub use signedref::SignedRef;

/// Reference to an oriented manifold in a [`Space`].
pub type ManifoldRef = SignedRef<ManifoldId>;
/// Reference to an oriented shape in a [`Space`].
pub type ShapeRef = SignedRef<ShapeId>;

hypermath::idx_struct! {
    /// ID for an unoriented manifold in a [`Space`].
    pub struct ManifoldId(pub u32);
    /// ID for an unoriented shape in a [`Space`].
    pub struct ShapeId(pub u32);
}

/// Space in which shapes can be constructed.
pub struct Space {
    /// Manifold of the entire space.
    manifold: ManifoldId,
    /// Shape covering the entire space.
    whole_space: ShapeId,

    /// Pseudoscalar blade.
    pseudoscalar: Blade,
    /// Inverse pseudoscalar blade.
    inverse_pseudoscalar: Blade,

    /// Submanifolds of the space.
    submanifolds: Slab<ManifoldData>,
    /// Lookup structure for submanifolds.
    submanifolds_hashmap: ApproxHashMap<Blade, ManifoldId>,

    /// Shapes defined in the space.
    shapes: Slab<ShapeData>,
    /// Lookup structure for shapes.
    shapes_hashmap: HashMap<ShapeData, ShapeId>,
}

impl fmt::Debug for Space {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Space")
            .field("ndim", &self.ndim())
            .finish_non_exhaustive()
    }
}

impl Index<ManifoldId> for Space {
    type Output = ManifoldData;

    fn index(&self, index: ManifoldId) -> &Self::Output {
        &self.submanifolds[index.0 as usize]
    }
}

impl Index<ShapeId> for Space {
    type Output = ShapeData;

    fn index(&self, index: ShapeId) -> &Self::Output {
        &self.shapes[index.0 as usize]
    }
}

impl Space {
    /// Constructs a new Euclidean space.
    pub fn new(ndim: u8) -> Self {
        let pseudoscalar = Blade::pseudoscalar(ndim);
        let inverse_pseudoscalar = Blade::inverse_pseudoscalar(ndim);

        let mut submanifolds = Slab::new();
        let mut submanifolds_hashmap = ApproxHashMap::new();
        let manifold_data =
            ManifoldData::new(pseudoscalar.clone()).expect("error constructing Euclidean space");
        let manifold = ManifoldId(submanifolds.insert(manifold_data) as u32);
        submanifolds_hashmap.insert(&pseudoscalar, manifold);

        let mut shapes = Slab::new();
        let mut shapes_hashmap = HashMap::new();
        let shape_data = ShapeData::whole_manifold(manifold);
        let whole_space = ShapeId(shapes.insert(shape_data) as u32);
        shapes_hashmap.insert(shapes[whole_space.0 as usize].clone(), whole_space);

        Space {
            manifold,
            whole_space,

            pseudoscalar,
            inverse_pseudoscalar,

            submanifolds,
            submanifolds_hashmap,

            shapes,
            shapes_hashmap,
        }
    }

    /// Returns the number of dimensions of the whole space.
    pub fn ndim(&self) -> u8 {
        self[self.manifold].ndim
    }
    /// Returns the manifold of the whole space.
    pub fn manifold(&self) -> ManifoldRef {
        self.manifold.into()
    }
    /// Returns a shape representing the whole space.
    pub fn whole_space(&self) -> ShapeRef {
        self.whole_space.into()
    }

    /// Returns the pseudoscalar of the space. This is useful when computing the
    /// intersection of two manifolds.
    pub fn pss(&self) -> &Blade {
        &self.pseudoscalar
    }
    /// Returns the inverse pseudoscalar of the space. This is useful when
    /// computing the intersection of two manifolds.
    pub fn inv_pss(&self) -> &Blade {
        &self.inverse_pseudoscalar
    }

    /// Returns the number of dimensions of a shape.
    pub fn ndim_of(&self, shape: ShapeId) -> u8 {
        self[self[shape].manifold].ndim
    }
    /// Returns the manifold of a shape.
    pub fn manifold_of(&self, shape: ShapeRef) -> ManifoldRef {
        ManifoldRef {
            id: self[shape.id].manifold,
            sign: shape.sign,
        }
    }
    /// Returns the blade representing a manifold.
    pub fn blade_of(&self, manifold: ManifoldRef) -> Blade {
        &self[manifold.id].blade * manifold.sign
    }
    /// Returns the signed boundary of a shape.
    pub fn boundary_of(&self, shape: ShapeRef) -> impl Iterator<Item = ShapeRef> {
        self[shape.id].boundary.iter().map(move |b| b * shape.sign)
    }

    /// Returns the pair of points that comprise a 0D shape.
    pub fn extract_point_pair(&self, shape: ShapeRef) -> Result<[Point; 2]> {
        let [a, b] = self[self[shape.id].manifold]
            .blade
            .point_pair_to_points()
            .context("attempt to get point pair from non-point point pair manifold")?;
        match shape.sign {
            Sign::Pos => Ok([a, b]),
            Sign::Neg => Ok([b, a]),
        }
    }

    /// Adds a spherical manifold to the space.
    pub fn add_sphere(&mut self, center: impl VectorRef, radius: Float) -> Result<ManifoldRef> {
        self.add_manifold(Blade::ipns_sphere(center, radius).ipns_to_opns_in_space(self.pss()))
    }
    /// Adds a planar manifold to the space.
    pub fn add_plane(&mut self, normal: impl VectorRef, distance: Float) -> Result<ManifoldRef> {
        self.add_manifold(Blade::ipns_plane(normal, distance).ipns_to_opns_in_space(self.pss()))
    }
    /// Adds a manifold to the space.
    pub fn add_manifold(&mut self, blade: Blade) -> Result<ManifoldRef> {
        ensure!(
            blade.ndim() <= self.ndim(),
            "manifold {blade} does not fit inside space",
        );
        ensure!(blade.grade() >= 2, "{blade} is too low-dimensional");

        // Canonicalize blade.
        let (blade, sign) = canonicalize_blade(blade)?;

        let manifold = match self.submanifolds_hashmap.entry(&blade) {
            hash_map::Entry::Occupied(e) => *e.get(),
            hash_map::Entry::Vacant(e) => {
                let manifold_data = ManifoldData::new(blade.clone())?;
                let key = self.submanifolds.insert(manifold_data);
                *e.insert(ManifoldId(key as u32))
            }
        };

        Ok(ManifoldRef { id: manifold, sign })
    }

    /// Adds a shape with no boundary to the space.
    pub fn add_shape_without_boundary(&mut self, manifold: ManifoldRef) -> Result<ShapeRef> {
        self.add_shape(manifold, ShapeSet::new())
    }
    /// Adds a shape to the space.
    fn add_shape(&mut self, manifold: ManifoldRef, boundary: ShapeSet) -> Result<ShapeRef> {
        // Canonicalize sign.
        let sign = manifold.sign;

        let shape_data = ShapeData {
            manifold: manifold.id,
            boundary: boundary * sign,
        };
        let manifold = self[shape_data.manifold].clone();

        let shape = *self
            .shapes_hashmap
            .entry(shape_data)
            .or_insert_with_key(|shape_data| {
                let key = self.shapes.insert(shape_data.clone());

                tracing::debug_span!("add_shape", id = %ShapeId(key as u32)).in_scope(|| {
                    tracing::debug!(ndim = manifold.ndim);
                    tracing::debug!(manifold = %manifold);
                    tracing::debug!(boundary = %shape_data.boundary);
                });

                ShapeId(key as u32)
            });

        #[cfg(debug_assertions)]
        self.sanity_check_shape(shape)?;

        Ok(ShapeRef { id: shape, sign })
    }

    /// Adds a shape using the manifold of an existing shape, or reuses the
    /// existing shape if possible. In particular, if `new_boundary` is the same
    /// as the boundary of `old_shape`, returns `old_shape`.
    #[tracing::instrument(skip_all, fields(old_shape = %old_shape, new_boundary = %new_boundary))]
    fn add_subshape(&mut self, old_shape: ShapeId, new_boundary: ShapeSet) -> Result<ShapeRef> {
        if new_boundary == self[old_shape].boundary {
            Ok(ShapeRef::from(old_shape))
        } else {
            let old_shape = &self[old_shape];
            self.add_shape(old_shape.manifold.into(), new_boundary)
        }
    }

    /// Runs basic sanity checks on a shape and returns an error if any fails.
    #[cfg(debug_assertions)]
    fn sanity_check_shape(&self, shape: ShapeId) -> Result<()> {
        let ndim = self.ndim_of(shape);

        // Check the rank of each boundary shape.
        for boundary_shape in &self[shape].boundary {
            ensure!(
                ndim == self.ndim_of(boundary_shape.id) + 1,
                "shape ndim does not match boundary ndim+1",
            );
        }

        // Check that polygons are topologically valid.
        if ndim == 2 {
            let mut starting_points = vec![];
            let mut ending_points = vec![];
            for edge in &self[shape].boundary {
                for point_pair in self.boundary_of(edge) {
                    let [a, b] = self.extract_point_pair(point_pair)?;

                    match ending_points.iter().find_position(|p| approx_eq(&a, p)) {
                        Some((i, _)) => {
                            ending_points.remove(i);
                        }
                        None => starting_points.push(a),
                    };
                    match starting_points.iter().find_position(|p| approx_eq(&b, p)) {
                        Some((i, _)) => {
                            starting_points.remove(i);
                        }
                        None => ending_points.push(b),
                    };
                }
            }
            if !(starting_points.is_empty() && ending_points.is_empty()) {
                tracing::error_span!("invalid polygon topology", %shape).in_scope(|| {
                    tracing::error!(boundary = %self[shape].boundary);
                    tracing::error!(starting_points = ?starting_points);
                    tracing::error!(ending_points = ?ending_points);
                });
                bail!("invalid polygon topology");
            }
        }

        Ok(())
    }

    /// Cuts a set of shapes, returning only the shapes on the inside of the
    /// cut.
    pub fn carve(&mut self, divider: ManifoldRef) -> CutInProgress<'_> {
        CutInProgress {
            space: self,
            op: CutOp::new(CutParams {
                divider,
                inside: ShapeFate::Keep,
                outside: ShapeFate::Remove,
            }),
        }
    }
    /// Cuts a set of shapes, returning shapes on both sides.
    pub fn slice(&mut self, divider: ManifoldRef) -> CutInProgress<'_> {
        CutInProgress {
            space: self,
            op: CutOp::new(CutParams {
                divider,
                inside: ShapeFate::Keep,
                outside: ShapeFate::Keep,
            }),
        }
    }

    /// Cuts a set of shapes by a manifold.
    #[tracing::instrument(skip_all)]
    pub fn cut(&mut self, shapes: &ShapeSet, params: CutParams) -> Result<(ShapeSet, ShapeSet)> {
        if params.inside == ShapeFate::Remove && params.outside == ShapeFate::Remove {
            // Why would you do this? You're just removing everything.
            return Ok((ShapeSet::new(), ShapeSet::new()));
        }

        let mut op = CutOp::new(params);
        let mut ret_inside = ShapeSet::new();
        let mut ret_outside = ShapeSet::new();
        for shape in shapes {
            match self.cut_shape(shape, &mut op)? {
                ShapeSplitResult::Flush => (), // Neither inside nor outside.
                ShapeSplitResult::ManifoldInside => {
                    ret_inside.insert(shape);
                }
                ShapeSplitResult::ManifoldOutside => {
                    ret_outside.insert(shape);
                }
                ShapeSplitResult::NonFlush {
                    inside,
                    outside,
                    intersection_shape: _,
                } => {
                    ret_inside.extend(inside);
                    ret_outside.extend(outside);
                }
            }
        }
        Ok((ret_inside, ret_outside))
    }

    /// Cuts a shape by a manifold.
    fn cut_shape(&mut self, shape: ShapeRef, op: &mut CutOp) -> Result<ShapeSplitResult> {
        if let Some(&cached_result) = op.shape_split_results_cache.get(&shape.id) {
            tracing::debug!("using cached split result for {}", shape.id);
            Ok(cached_result * shape.sign)
        } else {
            let result = self
                .cut_shape_uncached(shape.id, op)
                .with_context(|| format!("error cutting shape {shape}"))?;

            op.shape_split_results_cache.insert(shape.id, result);
            Ok(result * shape.sign)
        }
    }

    /// Cuts a shape by a manifold without first checking the shape results
    /// cache.
    #[tracing::instrument(skip_all, fields(shape = %shape), ret(Display), err(Debug))]
    fn cut_shape_uncached(&mut self, shape: ShapeId, op: &mut CutOp) -> Result<ShapeSplitResult> {
        let shape_manifold = self[shape].manifold;

        match op.cached_which_side_of_cut_contains_manifold(self, shape_manifold)? {
            ManifoldWhichSide::Flush => Ok(ShapeSplitResult::Flush),
            ManifoldWhichSide::Inside => Ok(ShapeSplitResult::ManifoldInside),
            ManifoldWhichSide::Outside => Ok(ShapeSplitResult::ManifoldOutside),
            ManifoldWhichSide::Split => {
                let intersection_manifold = op.cached_intersection_of_manifold_and_cut(
                    self,
                    self.manifold_of(shape.into()),
                )?;

                // The shape's manifold is split! Let's find out if the shape
                // itself is split too.

                // 1D manifolds may have a disconnected boundary, so they
                // require special handling.
                if self.ndim_of(shape) == 1 {
                    self.cut_split_shape_1d(shape, op, intersection_manifold)
                } else {
                    self.cut_split_shape_nd(shape, op, intersection_manifold)
                }
            }
        }
    }

    #[tracing::instrument(skip_all, fields(shape = %shape))]
    fn cut_split_shape_1d(
        &mut self,
        shape: ShapeId,
        op: &mut CutOp,
        intersection_manifold: ManifoldRef,
    ) -> Result<ShapeSplitResult> {
        let shape_manifold = self[shape].manifold;

        let intersection_shape = self.add_shape(intersection_manifold, ShapeSet::new())?;

        // Simplify inside boundary.
        let inside = match op.cut.inside {
            ShapeFate::Remove => None,
            ShapeFate::Keep => {
                let inside_boundary = self.incrementally_simplify_intersection_of_intervals(
                    shape_manifold.into(),
                    self[shape].boundary.clone(),
                    intersection_shape,
                )?;
                if let Some(boundary) = inside_boundary {
                    Some(self.add_subshape(shape, boundary)?)
                } else {
                    None // There is no outside shape.
                }
            }
        };

        // Simplify outside boundary.
        let outside = match op.cut.outside {
            ShapeFate::Remove => None,
            ShapeFate::Keep => {
                let outside_boundary = self.incrementally_simplify_intersection_of_intervals(
                    shape_manifold.into(),
                    self[shape].boundary.clone(),
                    -intersection_shape,
                )?;
                if let Some(boundary) = outside_boundary {
                    Some(self.add_subshape(shape, boundary)?)
                } else {
                    None // There is no inside shape.
                }
            }
        };

        Ok(ShapeSplitResult::NonFlush {
            inside,
            outside,
            intersection_shape: Some(intersection_shape),
        })
    }

    #[tracing::instrument(skip_all, fields(shape = %shape))]
    fn cut_split_shape_nd(
        &mut self,
        shape: ShapeId,
        op: &mut CutOp,
        intersection_manifold: ManifoldRef,
    ) -> Result<ShapeSplitResult> {
        let shape_boundary = self[shape].boundary.clone();

        // First, scan for any boundary shape that is exactly on the cut.
        for child in &shape_boundary {
            if self[child.id].manifold == intersection_manifold.id {
                tracing::debug!("found flush child {}", child);

                // The child is flush with the cut, so `shape` is not split.
                // `shape` is either on one side or the other or it's flush.
                let sign = child.sign * intersection_manifold.sign;
                return Ok(ShapeSplitResult::NonFlush {
                    inside: (sign == Sign::Pos && op.cut.inside == ShapeFate::Keep)
                        .then_some(shape.into()),
                    outside: (sign == Sign::Neg && op.cut.outside == ShapeFate::Keep)
                        .then_some(shape.into()),
                    intersection_shape: Some(child * sign),
                });
            }
        }

        // Next, scan for any boundary shape that is completely contained inside
        // the cut.
        for child in &shape_boundary {
            // Which side of the cut contains the child?
            let which_side_of_cut_contains_child =
                op.cached_which_side_of_cut_contains_manifold(self, self[child.id].manifold)?;
            match which_side_of_cut_contains_child {
                ManifoldWhichSide::Flush => bail!("manifold is flush, but has different ID"),
                ManifoldWhichSide::Split => continue,
                _ => (),
            }

            // Which side of the *child* contains the *cut*?
            if self.which_side(
                ManifoldRef::from(self[shape].manifold),
                self.manifold_of(child),
                intersection_manifold.id,
            )? == ManifoldWhichSide::Outside
            {
                tracing::debug!("found child {} that excludes cut", child);

                // Based on just this child, the cut is completely outside
                // `shape`. So we know that `shape` is either completely inside
                // the cut or completely outside the cut.
                let mut inside = None;
                let mut outside = None;
                match which_side_of_cut_contains_child {
                    ManifoldWhichSide::Flush | ManifoldWhichSide::Split => {
                        unreachable!("cases already handled")
                    }
                    ManifoldWhichSide::Inside => inside = Some(ShapeRef::from(shape)),
                    ManifoldWhichSide::Outside => outside = Some(ShapeRef::from(shape)),
                }
                return Ok(ShapeSplitResult::NonFlush {
                    inside,
                    outside,
                    intersection_shape: None,
                });
            }
        }

        // Alright, we've handled the edge cases. Now for the general case:
        // `shape` may be inside, outside, or split.

        // Let `cut` represent the inside half-space of the cut, so `~cut` is
        // the outside half-space and `boundary(cut)` is the whole cut manifold.

        // (N-1)-dimensional shapes that together comprise `boundary(shape ∩
        // cut)`, or equivalently, `boundary(shape) ∩ cut`.
        let mut self_boundary_of_inside = ShapeSet::new();

        // (N-1)-dimensional shapes that together comprise `boundary(shape ∩
        // ~cut)`, or equivalently, `boundary(shape) ∩ ~cut`.
        let mut self_boundary_of_outside = ShapeSet::new();

        // (N-2)-dimensional shapes that together comprise `boundary(shape ∩
        // boundary(cut))`, or equivalently, `boundary(shape) ∩ boundary(cut)`.
        let mut intersection_boundary = ShapeSet::new();

        // Split each of the "child" shapes that comprise `boundary(shape)`.
        for child in &shape_boundary {
            match self.cut_shape(child, op)? {
                ShapeSplitResult::Flush => bail!("manifold is flush, but has different ID"),
                ShapeSplitResult::ManifoldInside => {
                    self_boundary_of_inside.insert(child);
                }
                ShapeSplitResult::ManifoldOutside => {
                    self_boundary_of_outside.insert(child);
                }
                ShapeSplitResult::NonFlush {
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

        tracing::trace!(self_boundary_of_inside = %self_boundary_of_inside);
        tracing::trace!(self_boundary_of_outside = %self_boundary_of_outside);
        tracing::trace!(intersection_boundary = %intersection_boundary);

        // Simplify boundary of intersection.
        intersection_boundary =
            self.simplify_shape_boundary(intersection_manifold, intersection_boundary)?;

        // Let `intersection_shape` be the (N-1)-dimensional shape that is
        // `shape ∩ boundary(cut)`.
        let mut intersection_shape = None;
        // There are two cases in which `intersection_shape` should be nonempty:
        // - `intersection_boundary` is nonempty, so `intersection_shape` should
        //   obviously be nonempty.
        // - The shape completely contains the manifold, so `intersection_shape`
        //   should be the entirety of `intersection_manifold` with no boundary.
        if !intersection_boundary.is_empty()
            || self.shape_completely_contains_manifold(shape, intersection_manifold.id)?
        {
            let new_shape = self.add_shape(intersection_manifold, intersection_boundary)?;

            // `shape ∩ boundary(cut)` is part of the boundary of `shape
            // ∩ cut` and part of the boundary of `shape ∩ ~cut`.
            self_boundary_of_inside.insert(new_shape);
            self_boundary_of_outside.insert(-new_shape);

            intersection_shape = Some(new_shape);
        }

        // Construct the N-dimensional shape that is `self ∩ cut`
        let mut inside = None;
        if op.cut.inside == ShapeFate::Keep && !self_boundary_of_inside.is_empty() {
            let s = tracing::info_span!("constructing inside shape")
                .in_scope(|| self.add_subshape(shape, self_boundary_of_inside))?;
            inside = Some(s);
        }

        // Construct the N-dimensional shape that is `self ∩ ~cut`
        let mut outside = None;
        if op.cut.outside == ShapeFate::Keep && !self_boundary_of_outside.is_empty() {
            let s = tracing::info_span!("constructing outside shape")
                .in_scope(|| self.add_subshape(shape, self_boundary_of_outside))?;
            outside = Some(s);
        };

        Ok(ShapeSplitResult::NonFlush {
            inside,
            outside,
            intersection_shape,
        })
    }

    /// Given N-dimensional `space` containing `target` and (N-1)-dimensional
    /// `cut`, returns whether `target` is at least partly contained by either
    /// half of `space` separated by `cut`.
    ///
    /// Which part of `space` is considered "inside" or "outside" depends on the
    /// orientations of `space` and `cut`. The orientation of `target` makes no
    /// difference.
    fn which_side(
        &self,
        space: ManifoldRef,
        cut: ManifoldRef,
        target: ManifoldId,
    ) -> Result<ManifoldWhichSide> {
        let sign = space.sign * cut.sign;

        let space = &self[space.id];
        let cut = &self[cut.id];
        let target = &self[target];

        ensure!(cut.ndim + 1 == space.ndim);

        if target.ndim == space.ndim {
            // `target` = `space`, and `cut` is a submanifold of `space`, so
            // `target` must be split.
            return Ok(ManifoldWhichSide::Split);
        }
        // Otherwise `target` must be a submanifold of `space`.
        ensure!(target.ndim < space.ndim);

        // Get the IPNS (inner product null space) representation of the
        // hypersphere that is perpendicular to `space` and tangent to `cut`.
        let cut_ipns = cut.blade.opns_to_ipns_in_space(&space.blade);
        // ... and the one tangent to `cut`.
        let target_ipns = target.blade.opns_to_ipns_in_space(&space.blade);

        // Find two points on `target` such that they straddle `cut` if `target`
        // intersects `cut`. If `target` is entirely on one side of `cut`, then
        // these points will both be on the same side.
        let pair_on_target_across_cut = if target.ndim == 0 {
            // `target` is a point pair. Just query each of those points.
            target.blade.clone()
        } else {
            // This algorithm took WEEKS of work to figure out. Huge thanks to
            // Luna Harran for helping!
            //
            // Here's a geometric algebra expression for what we're about to do:
            // `c1 & !(c1 & c2 & !p7)`
            //
            // See `manifold_which_side_demo.js` for an interactive ganja.js demo.

            // 1. Compute the dual of the intersection of `target` and `cut`. I
            //    think this represents a bundle of all the manifolds that are
            //    perpendicular to `target` and `cut`.
            let perpendicular_bundle = &target_ipns ^ &cut_ipns;

            if perpendicular_bundle.is_zero() {
                return Ok(ManifoldWhichSide::Flush);
            }

            // 2. Wedge with an arbitrary point to select one of those possible
            //    perpendicular manifolds. The only restriction here is that we
            //    don't want the wedge product to be zero.
            let perpendicular_manifold = nonzero_wedge_with_arbitrary_point(&perpendicular_bundle)?;

            // 3. Intersect that perpendicular manifold with `target` to get two
            //    points on `target`.
            (target_ipns ^ perpendicular_manifold.opns_to_ipns_in_space(&space.blade))
                .ipns_to_opns_in_space(&space.blade)

            // There exists some conformal transformation `C` that turns
            // `perpendicular_manifold` into a flat line/plane/hyperplane and
            // make `target` and `cut` both circles/spheres/hyperspheres
            // perpendicular to it.
            //
            // `pair_on_target_across_cut` is the intersection of `target` and
            // `perpendicular_manifold`.
            //
            // After applying `C`, `pair_on_target_across_cut` consists of the
            // two points on `target` that are closest and farthest from
            // `perpendicular_manifold`. If any point on `target` is inside
            // `cut`, then the closest point will also be inside `cut`. And if
            // any point on `target` is outside `cut`, then the farthest point
            // will also be outside `cut`.
        };

        // Extract those two points.
        //
        // If the manifolds are just barely tangent, then
        // `pair_on_target_across_cut` will be degenerate. Pick any two points
        // on the manifold, as long as they aren't the same, so that at most one
        // of them could be the tangent point.
        let [a, b] = match pair_on_target_across_cut.point_pair_to_points() {
            Some(pair) => pair,
            None => find_arbitrary_point_pair_on_container(&target.blade, self.ndim())?,
        };

        // Query whether each one is inside or outside of `cut`.
        Ok(ManifoldWhichSide::from_points([
            cut_ipns.ipns_query_point(&a),
            cut_ipns.ipns_query_point(&b),
        ]) * sign)
    }

    /// Given the N-dimensional `space` containing (N-1)-dimensional `cut` and
    /// M-dimensional `target` where M<=N, returns the (M-1)-dimensional
    /// intersection of `target` and `cut`. If `target` and `cut` do not
    /// intersect or if any of the other preconditions are broken, this function
    /// may return an error or garbage.
    ///
    /// The orientation of the result depends on the orientations of `space`,
    /// `cut`, and `target`.
    fn intersect(
        &mut self,
        space: ManifoldRef,
        cut: ManifoldRef,
        target: ManifoldRef,
    ) -> Result<ManifoldRef> {
        let sign = space.sign * cut.sign * target.sign;

        if target.id == space.id {
            // Optimization: `target` is the whole space, so the intersection
            // between `target` and `cut` is just `cut`.
            return Ok(ManifoldRef { id: cut.id, sign });
        }

        let space = &self[space.id];
        let cut = &self[cut.id];
        let target = &self[target.id];

        ensure!(cut.ndim + 1 == space.ndim);
        ensure!(target.ndim <= space.ndim);

        // Compute a "meet" which is the dual of the outer product.
        let cut_ipns = cut.blade.opns_to_ipns_in_space(&space.blade);
        let target_ipns = target.blade.opns_to_ipns_in_space(&space.blade);
        let intersection = (cut_ipns ^ target_ipns).ipns_to_opns_in_space(&space.blade);
        ensure!(
            intersection.opns_is_real(),
            "intersection {intersection} is not real",
        );

        Ok(self.add_manifold(intersection)? * sign)
    }

    fn simplify_shape_boundary(
        &mut self,
        manifold: ManifoldRef,
        boundary: ShapeSet,
    ) -> Result<ShapeSet> {
        // This method is slightly questionable, since its return value doesn't
        // indicate the case where simplifying the boundary revealed that the
        // shape cannot exist. It's not an issue in practice because it's only
        // ever called in a context where we can check through other means
        // whether the shape should exist, even if it has no boundary.
        if self[manifold.id].ndim == 1 {
            Ok(self
                .simplify_intersection_of_intervals(manifold, &boundary)
                .context("error simplifying boundary of 1D intersection")?
                .unwrap_or_else(ShapeSet::new))
        } else {
            // Just remove duplicates (which `Set64` does automatically for us)
            // and cancel opposite signs.
            Ok(boundary
                .iter()
                .filter(|&elem| !boundary.0.contains(-elem))
                .collect())
        }
    }

    /// Simplifies a subset of a 1D manifold represented as the intersection of
    /// a set of intervals, where each interval is represented as a point pair.
    ///
    /// Returns `None` if the intersection is empty.
    fn simplify_intersection_of_intervals(
        &mut self,
        space: ManifoldRef,
        intervals: &ShapeSet,
    ) -> Result<Option<ShapeSet>> {
        let mut simplified = ShapeSet::new();
        for interval in intervals {
            match self
                .incrementally_simplify_intersection_of_intervals(space, simplified, interval)?
            {
                Some(b) => simplified = b,
                None => return Ok(None),
            }
        }
        Ok(Some(simplified))
    }
    fn incrementally_simplify_intersection_of_intervals(
        &mut self,
        space: ManifoldRef,
        existing_intervals: ShapeSet,
        mut new_interval: ShapeRef,
    ) -> Result<Option<ShapeSet>> {
        let mut simplified = ShapeSet::new();
        for existing_interval in existing_intervals {
            // The intersection of intervals is the complement of the union of
            // the complements. (Negating a point pair corresponds to taking the
            // complement of an interval.)
            match self.try_union_intervals(space, -existing_interval, -new_interval)? {
                MergedInterval::Merged(shape) => new_interval = -shape,

                MergedInterval::WholeSpace => return Ok(None), // whole space is excluded; there's nothing left

                MergedInterval::NoIntersection => {
                    simplified.insert(existing_interval);
                }
            }
        }
        simplified.insert(new_interval);

        // Check that all points are unique.
        if cfg!(debug_assertions) {
            let mut verts: Vec<Point> = simplified
                .iter()
                .map(|s| self.extract_point_pair(s))
                .flatten_ok()
                .try_collect()
                .expect("invalid point pairs");
            while let Some(v1) = verts.pop() {
                for v2 in &verts {
                    assert!(!approx_eq(&v1, v2));
                }
            }
        }

        Ok(Some(simplified))
    }
    /// If two intervals (including their boundaries) intersect at all, returns
    /// the interval that is their union. Otherwise returns `None`.
    fn try_union_intervals(
        &mut self,
        space: ManifoldRef,
        interval1: ShapeRef,
        interval2: ShapeRef,
    ) -> Result<MergedInterval> {
        let [a, b] = self.extract_point_pair(interval1)?;
        let [p, q] = self.extract_point_pair(interval2)?;
        let ab = ManifoldRef::from(self[interval1.id].manifold) * interval1.sign;
        let pq = ManifoldRef::from(self[interval2.id].manifold) * interval2.sign;

        let start;
        let end;

        if approx_eq(&a, &p) && approx_eq(&b, &q) {
            // The intervals are the same.
            start = &a; // equivalent to `p`
            end = &b; // equivalent to `q`
        } else {
            let ab_has_p = self.closed_interval_contains_point(space, ab, &p);
            let ab_has_q = self.closed_interval_contains_point(space, ab, &q);
            let pq_has_a = self.closed_interval_contains_point(space, pq, &a);
            let pq_has_b = self.closed_interval_contains_point(space, pq, &b);
            let ab_has_pq = ab_has_p && ab_has_q;
            let pq_has_ab = pq_has_a && pq_has_b;

            if ab_has_pq && pq_has_ab {
                return Ok(MergedInterval::WholeSpace);
            }

            start = if ab_has_p {
                &a
            } else if pq_has_a {
                &p
            } else {
                return Ok(MergedInterval::NoIntersection);
            };

            end = if ab_has_q {
                &b
            } else if pq_has_b {
                &q
            } else {
                return Ok(MergedInterval::NoIntersection);
            };
        }

        let new_point_pair_manifold =
            self.add_manifold(start.to_normalized_1blade() ^ end.to_normalized_1blade())?;
        let new_point_pair_shape = self.add_shape(new_point_pair_manifold, ShapeSet::new())?;
        Ok(MergedInterval::Merged(new_point_pair_shape))
    }
    /// Returns whether the `interval` represented by a point pair within a 1D
    /// `space` contains `point`. The interval is considered to **include** its
    /// boundary.
    fn closed_interval_contains_point(
        &self,
        space: ManifoldRef,
        interval: ManifoldRef,
        point: &Point,
    ) -> bool {
        self.which_side_has_point(space, interval, point) != PointWhichSide::Outside
    }

    /// Returns whether `manifold` (which is assumed to be a submanifold of the
    /// manifold of `shape`) is completely inside `shape`.
    ///
    /// To be considered "completely inside," `manifold` may only touch the
    /// boundary of `shape` at finitely many points. In other words, it can be
    /// tangent to the boundary of `shape` but not flush with a boundary
    /// element.
    fn shape_completely_contains_manifold(
        &self,
        shape: ShapeId,
        manifold: ManifoldId,
    ) -> Result<bool> {
        let shape_manifold = self[shape].manifold;
        for boundary_elem in self[shape].boundary.iter() {
            match self.which_side(
                shape_manifold.into(),
                self.manifold_of(boundary_elem),
                manifold,
            )? {
                ManifoldWhichSide::Inside => continue,
                ManifoldWhichSide::Flush
                | ManifoldWhichSide::Outside
                | ManifoldWhichSide::Split => return Ok(false),
            }
        }
        Ok(true)
    }

    /// Returns whether the inside or outside of `cut` contains `p`, within
    /// `space`.
    pub fn which_side_has_point(
        &self,
        space: ManifoldRef,
        cut: ManifoldRef,
        p: &Point,
    ) -> PointWhichSide {
        self[cut.id]
            .blade
            .opns_to_ipns_in_space(&self[space.id].blade)
            .ipns_query_point(p)
            * (space.sign * cut.sign)
    }

    /// Outputs a string representation of a shape for debugging.
    pub fn shape_to_string(&self, shape: ShapeRef) -> String {
        let mut buffer = String::new();
        self.shape_to_string_internal(&mut buffer, shape, 0);
        buffer
    }
    fn shape_to_string_internal(&self, buffer: &mut String, shape: ShapeRef, indent: u8) {
        for _ in 0..indent {
            *buffer += "  ";
        }
        *buffer += &format!("{}#{:<5}", shape.sign, shape.id.0);
        let manifold = self.manifold_of(shape);
        let blade = &self[manifold.id].blade * manifold.sign;
        if self[manifold.id].ndim == 0 {
            let [a, b] = blade.point_pair_to_points().expect("bad point pair");
            *buffer += &format!("{a}..{b}");
        } else {
            *buffer += &blade.to_string();
        }
        buffer.push('\n');
        for child in self.boundary_of(shape) {
            self.shape_to_string_internal(buffer, child, indent + 1);
        }
    }
}

/// Normalizes a blade and canonicalizes it, returning the sign difference
/// between the blade and its canonicalization.
fn canonicalize_blade(blade: Blade) -> Result<(Blade, Sign)> {
    // Normalize with respect to the most significant term, so that the blade is
    // approximately normalized. We don't actually care what its magnitude and
    // sign are, just that it has a consistent magnitude and sign regardless of
    // slight variations caused by float imprecision.
    let scale_factor = blade.mv().most_significant_term().coef.abs().recip();

    // Determine the sign based on the first nonzero term, which is robust
    // against slight variations caused by float imprecision.
    let first_term = blade.mv().nonzero_terms().next();
    let sign = Sign::from(first_term.context("zero manifold is not valid")?.coef);

    Ok((blade * (scale_factor * sign), sign))
}

/// Selects an arbitrary point that is not on the object and wedges the object
/// with that point.
///
/// Returns an error if there is no such point, which should only happen if the
/// object is already zero.
fn nonzero_wedge_with_arbitrary_point(opns_blade: &Blade) -> Result<Blade> {
    let ndim = opns_blade.ndim() + 1;
    let candidates = (0..ndim)
        .map(|i| Blade::point(Vector::unit(i)))
        .chain([Blade::NO, Blade::NI]);
    candidates
        .map(|p| opns_blade ^ p)
        .max_by_key(|obj| FloatOrd(obj.abs_mag2()))
        .ok_or_else(|| anyhow!("unable to find point not on object {opns_blade}"))
}

/// Returns an arbitrary pair of points on the [container] of the manifold.
///
/// [container]:
///     http://conformalgeometricalgebra.org/wiki/index.php?title=Containers
fn find_arbitrary_point_pair_on_container(
    opns_blade: &Blade,
    space_ndim: u8,
) -> Result<[Point; 2]> {
    let ipns = opns_blade.opns_to_ipns(space_ndim);
    if let Some(radius) = ipns.ipns_radius() {
        let center = ipns
            .ipns_sphere_center()
            .to_finite()
            .map_err(|_| anyhow!("error computing center of sphere"))?;
        Ok([
            Point::Finite(vector![radius] + &center),
            Point::Finite(vector![-radius] + &center),
        ])
    } else {
        Ok([Point::Finite(ipns.ipns_plane_pole()), Point::Infinity])
    }
}
