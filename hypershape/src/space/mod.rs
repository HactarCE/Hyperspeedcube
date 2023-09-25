//! Infinite Euclidean space in which polytopes can be constructed.

use std::cmp::Ordering;
use std::collections::{hash_map, HashMap};
use std::fmt;
use std::ops::{Index, Mul, MulAssign, Neg};

use anyhow::{anyhow, bail, ensure, Context, Result};
use float_ord::FloatOrd;
use hypermath::prelude::*;
use itertools::Itertools;
use tinyset::Set64;

mod atomic_polytope;
mod cut;
mod manifold;
mod results;
mod signedref;

pub use atomic_polytope::AtomicPolytope;
pub use cut::{Cut, CutParams, PolytopeFate};
pub use manifold::ManifoldData;
use results::{AtomicPolytopeCutOutput, IntervalUnion, ManifoldWhichSide};
pub use signedref::SignedRef;

use crate::SlabMap;

/// Reference to an oriented manifold in a [`Space`].
pub type ManifoldRef = SignedRef<ManifoldId>;
/// Reference to an oriented atomic polytope in a [`Space`].
pub type AtomicPolytopeRef = SignedRef<AtomicPolytopeId>;

/// Set of oriented atomic polytopes in a [`Space`].
pub type AtomicPolytopeSet = Set64<AtomicPolytopeRef>;

hypermath::idx_struct! {
    /// ID for a memoized unoriented manifold in a [`Space`].
    pub struct ManifoldId(pub u32);
    /// ID for a memoized unoriented atomic polytope in a [`Space`].
    pub struct AtomicPolytopeId(pub u32);
}

/// Euclidean space in which polytopes can be constructed.
pub struct Space {
    /// Submanifolds of the space.
    manifolds: SlabMap<ManifoldId, ManifoldData>,
    /// Atomic polytopes defined in the space.
    polytopes: SlabMap<AtomicPolytopeId, AtomicPolytope>,

    /// Manifold of the entire space.
    covering_manifold: ManifoldId,
    /// Polytope with no border covering the entire space.
    covering_polytope: AtomicPolytopeId,
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
        &self.manifolds[index]
    }
}

impl Index<AtomicPolytopeId> for Space {
    type Output = AtomicPolytope;

    fn index(&self, index: AtomicPolytopeId) -> &Self::Output {
        &self.polytopes[index]
    }
}

impl Space {
    /// Constructs a new Euclidean space.
    pub fn new(ndim: u8) -> Result<Self> {
        let mut manifolds = SlabMap::new();
        let mut polytopes = SlabMap::new();

        let pseudoscalar = Blade::pseudoscalar(ndim);
        let covering_manifold = manifolds
            .get_or_insert(ManifoldData::new(pseudoscalar)?)?
            .key();
        let covering_polytope = polytopes
            .get_or_insert(AtomicPolytope::whole_manifold(covering_manifold))?
            .key();

        Ok(Space {
            manifolds,
            polytopes,

            covering_manifold,
            covering_polytope,
        })
    }

    /// Returns the number of dimensions of the whole space.
    pub fn ndim(&self) -> u8 {
        self[self.covering_manifold].ndim
    }
    /// Returns the manifold of the whole space.
    pub fn manifold(&self) -> ManifoldRef {
        self.covering_manifold.into()
    }
    /// Returns the polytope representing the whole space.
    pub fn whole_space(&self) -> AtomicPolytopeRef {
        self.covering_polytope.into()
    }

    /// Returns the pseudoscalar of the space. This is useful when computing the
    /// intersection of two manifolds.
    pub fn pss(&self) -> &Blade {
        &self[self.covering_manifold].blade
    }

    /// Returns the number of dimensions of a manifold or polytope.
    pub fn ndim_of(&self, thing: impl HasManifoldInSpace) -> u8 {
        self[self.manifold_of(thing).id].ndim
    }
    /// Returns the manifold of a polytope.
    pub fn manifold_of(&self, thing: impl HasManifoldInSpace) -> ManifoldRef {
        thing.get_manifold_ref(self)
    }
    /// Returns the blade representing a manifold or a polytope's manifold.
    pub fn blade_of(&self, thing: impl HasManifoldInSpace) -> Blade {
        let m = self.manifold_of(thing);
        &self[m.id].blade * m.sign
    }
    /// Returns the signed boundary of a polytope.
    pub fn boundary_of(
        &self,
        polytope: impl Into<AtomicPolytopeRef>,
    ) -> impl '_ + Iterator<Item = AtomicPolytopeRef> {
        let polytope = polytope.into();
        self[polytope.id]
            .boundary
            .iter()
            .map(move |boundary_elem| boundary_elem * polytope.sign)
    }

    /// Returns the pair of points that comprise a 0D polytope.
    pub fn extract_point_pair(&self, polytope: AtomicPolytopeRef) -> Result<[Point; 2]> {
        self.blade_of(polytope)
            .point_pair_to_points()
            .context("attempt to get point pair from non-point point pair manifold")
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
        // Canonicalize blade.
        let (blade, sign) = canonicalize_blade(blade)?;

        let manifold_data = ManifoldData::new(blade)?;
        ensure!(
            manifold_data.ndim <= self.ndim(),
            "manifold {manifold_data} does not fit inside space",
        );
        let manifold_id = self.manifolds.get_or_insert(manifold_data)?.key();

        Ok(manifold_id * sign)
    }

    /// Adds a point pair to the space.
    fn add_point_pair(&mut self, manifold: ManifoldRef) -> Result<AtomicPolytopeRef> {
        ensure!(
            self.ndim_of(manifold) == 0,
            "add_point_pair() requires ndim = 0",
        );

        let polytope_data = AtomicPolytope::whole_manifold(manifold.id);
        let polytope_id = self.get_or_insert_polytope_data(polytope_data)?;
        Ok(polytope_id * manifold.sign)
    }
    /// Adds an atomic polytope to the space.
    fn add_atomic_polytope(
        &mut self,
        manifold: ManifoldRef,
        boundary: AtomicPolytopeSet,
    ) -> Result<AtomicPolytopeRef> {
        ensure!(
            self.ndim_of(manifold) > 0,
            "add_atomic_polytope() requires ndim > 0",
        );

        let polytope_data = AtomicPolytope::new(manifold.id, boundary);
        let polytope_id = self.get_or_insert_polytope_data(polytope_data)?;
        Ok(polytope_id * manifold.sign)
    }
    /// Adds an atomic polytope using the manifold of an existing polytope, or
    /// reuses the existing polytope if possible. In particular, if
    /// `new_boundary` is the same as the boundary of `old_polytope`, then this
    /// method returns `old_polytope`; otherwise it creates and returns a new
    /// polytope.
    #[tracing::instrument(skip_all, fields(%old_polytope))]
    fn add_atomic_subpolytope(
        &mut self,
        old_polytope: AtomicPolytopeId,
        new_boundary: AtomicPolytopeSet,
    ) -> Result<AtomicPolytopeId> {
        let old_polytope_data = &self[old_polytope];
        let mut new_polytope_data = old_polytope_data.clone();
        new_polytope_data.boundary = new_boundary;

        if *old_polytope_data == new_polytope_data {
            // Reuse the old polytope.
            Ok(old_polytope)
        } else {
            // Make a new polytope.
            self.get_or_insert_polytope_data(new_polytope_data)
        }
    }
    /// Returns the ID of an atomic polytope with certain data, adding it to the
    /// space if it does not already exist.
    #[tracing::instrument(skip_all, fields(polytope_data))]
    fn get_or_insert_polytope_data(
        &mut self,
        polytope_data: AtomicPolytope,
    ) -> Result<AtomicPolytopeId> {
        let ndim = self.ndim_of(&polytope_data);

        let entry = self.polytopes.get_or_insert(polytope_data.clone())?;
        let polytope = entry.key();
        if entry.is_new() {
            // Log a bunch of a stuff for debugging.
            tracing::debug_span!("add_polytope", id = ?polytope).in_scope(|| {
                tracing::debug!(ndim);
                tracing::debug!(data = ?polytope_data);
            });

            // Run basic sanity checks.
            #[cfg(debug_assertions)]
            self.sanity_check_atomic_polytope(polytope)?;
        }

        Ok(polytope)
    }

    /// Runs basic sanity checks on an atomic polytope and returns an error if
    /// any fails.
    #[cfg(debug_assertions)]
    fn sanity_check_atomic_polytope(&self, polytope: AtomicPolytopeId) -> Result<()> {
        let ndim = self.ndim_of(polytope);

        // Check the rank of each boundary polytope.
        for boundary_elem in self.boundary_of(polytope) {
            ensure!(
                ndim == self.ndim_of(boundary_elem.id) + 1,
                "polytope ndim does not match boundary ndim+1",
            );
        }

        // Check that polygons are topologically valid.
        if ndim == 2 {
            let mut starting_points = vec![];
            let mut ending_points = vec![];
            for edge in self.boundary_of(polytope) {
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
                tracing::error_span!("invalid polygon topology", %polytope).in_scope(|| {
                    tracing::error!(polytope_data = ?self[polytope]);
                    tracing::error!(?starting_points);
                    tracing::error!(?ending_points);
                });
                bail!("invalid polygon topology");
            }
        }

        Ok(())
    }

    /// Cuts an atomic polytope by a manifold.
    #[tracing::instrument(skip_all, fields(%polytope, ?cut))]
    pub fn cut_atomic_polytope(
        &mut self,
        polytope: AtomicPolytopeRef,
        cut: &mut Cut,
    ) -> Result<AtomicPolytopeCutOutput> {
        if let Some(&cached_result) = cut.polytope_cut_output_cache.get(&polytope.id) {
            tracing::debug!("using cached cut output for {}", polytope.id);
            Ok(cached_result * polytope.sign)
        } else {
            let result = self
                .cut_atomic_polytope_uncached(polytope.id, cut)
                .with_context(|| format!("error cutting polytope {polytope}"))?;

            cut.polytope_cut_output_cache.insert(polytope.id, result);
            Ok(result * polytope.sign)
        }
    }

    /// Cuts an atomic polytope by a manifold without first checking the cache.
    #[tracing::instrument(skip_all, fields(%polytope), ret(Display), err(Debug))]
    fn cut_atomic_polytope_uncached(
        &mut self,
        polytope: AtomicPolytopeId,
        cut: &mut Cut,
    ) -> Result<AtomicPolytopeCutOutput> {
        match cut.which_side_of_cut_contains_manifold(self, self.manifold_of(polytope).id)? {
            ManifoldWhichSide::Flush => Ok(AtomicPolytopeCutOutput::Flush),
            ManifoldWhichSide::Inside => Ok(AtomicPolytopeCutOutput::ManifoldInside),
            ManifoldWhichSide::Outside => Ok(AtomicPolytopeCutOutput::ManifoldOutside),
            ManifoldWhichSide::Split => {
                let intersection_manifold =
                    cut.intersection_of_manifold_and_cut(self, self.manifold_of(polytope))?;

                // The polytopes's manifold is split! Let's find out if the
                // polytope itself is split too.

                // 1D manifolds may have a disconnected boundary, so they
                // require special handling.
                match self.ndim_of(polytope) {
                    1 => self.cut_atomic_polytope_1d(polytope, cut, intersection_manifold),
                    _ => self.cut_atomic_polytope_nd(polytope, cut, intersection_manifold),
                }
            }
        }
    }

    /// Cuts a 1D atomic polytope, assuming it is split by the cut.
    #[tracing::instrument(skip_all, fields(%polytope))]
    fn cut_atomic_polytope_1d(
        &mut self,
        polytope: AtomicPolytopeId,
        cut: &mut Cut,
        intersection_manifold: ManifoldRef,
    ) -> Result<AtomicPolytopeCutOutput> {
        let polytope_manifold = self.manifold_of(polytope);
        let polytope_boundary = self.boundary_of(polytope).collect_vec();

        // The intersection polytope is 0D, so it has no boundary.
        let intersection = self.add_point_pair(intersection_manifold)?;

        // Simplify inside boundary.
        let inside = match cut.params.inside {
            PolytopeFate::Remove => None,
            PolytopeFate::Keep => {
                let inside_boundary = self.incrementally_simplify_intersection_of_intervals(
                    polytope_manifold,
                    polytope_boundary.iter().copied(),
                    intersection,
                )?;
                if let Some(boundary) = inside_boundary {
                    Some(self.add_atomic_subpolytope(polytope, boundary)?)
                } else {
                    None // There is no outside polytope.
                }
            }
        };

        // Simplify outside boundary.
        let outside = match cut.params.outside {
            PolytopeFate::Remove => None,
            PolytopeFate::Keep => {
                let outside_boundary = self.incrementally_simplify_intersection_of_intervals(
                    polytope_manifold,
                    polytope_boundary.iter().copied(),
                    -intersection,
                )?;
                if let Some(boundary) = outside_boundary {
                    Some(self.add_atomic_subpolytope(polytope, boundary)?)
                } else {
                    None // There is no inside polytope.
                }
            }
        };

        Ok(AtomicPolytopeCutOutput::NonFlush {
            inside: inside.map(AtomicPolytopeRef::from),
            outside: outside.map(AtomicPolytopeRef::from),
            intersection: Some(intersection),
        })
    }

    /// Cuts an N-dimensional atomic polytope, assuming it is split by the cut.
    #[tracing::instrument(skip_all, fields(%polytope))]
    fn cut_atomic_polytope_nd(
        &mut self,
        polytope: AtomicPolytopeId,
        cut: &mut Cut,
        intersection_manifold: ManifoldRef,
    ) -> Result<AtomicPolytopeCutOutput> {
        let polytope_boundary = self.boundary_of(polytope).collect_vec();
        let polytope_ref = AtomicPolytopeRef::from(polytope);

        // First, scan for any boundary polytope that is exactly on the cut.
        for child in self.boundary_of(polytope) {
            let child_manifold = self.manifold_of(child);
            if child_manifold.id == intersection_manifold.id {
                tracing::debug!("found flush child {child}");

                // The child is flush with the cut, so `polytope` is not split.
                // `polytope` is either on one side or the other or it's flush.
                let sign = child_manifold.sign * intersection_manifold.sign;
                return Ok(AtomicPolytopeCutOutput::NonFlush {
                    inside: (sign == Sign::Pos && cut.params.inside == PolytopeFate::Keep)
                        .then_some(polytope_ref),
                    outside: (sign == Sign::Neg && cut.params.outside == PolytopeFate::Keep)
                        .then_some(polytope_ref),
                    intersection: Some(child * sign),
                });
            }
        }

        // Next, scan for any boundary polytope whose manifold is completely
        // contained inside the cut.
        for &child in &polytope_boundary {
            // Which side of the cut contains the child?
            let child_manifold = self.manifold_of(child).id;
            let which_side_of_cut_contains_child =
                cut.which_side_of_cut_contains_manifold(self, child_manifold)?;
            match which_side_of_cut_contains_child {
                ManifoldWhichSide::Flush => bail!("manifold is flush, but has different ID"),
                ManifoldWhichSide::Split => continue,
                _ => (),
            }

            // Which side of the *child* contains the *cut*?
            if self.which_side(
                self.manifold_of(polytope),
                self.manifold_of(child),
                intersection_manifold.id,
            )? == ManifoldWhichSide::Outside
            {
                tracing::debug!("found child {child} that excludes cut");

                // Based on just this child, the cut is completely outside
                // `polytope`. So we know that `polytope` is either completely
                // inside the cut or completely outside the cut.
                let mut inside = None;
                let mut outside = None;
                match which_side_of_cut_contains_child {
                    ManifoldWhichSide::Flush | ManifoldWhichSide::Split => {
                        unreachable!("cases already handled")
                    }
                    ManifoldWhichSide::Inside => inside = Some(AtomicPolytopeRef::from(polytope)),
                    ManifoldWhichSide::Outside => outside = Some(AtomicPolytopeRef::from(polytope)),
                }
                return Ok(AtomicPolytopeCutOutput::NonFlush {
                    inside,
                    outside,
                    intersection: None,
                });
            }
        }

        // Alright, we've handled the edge cases. Now for the general case:
        // `polytope` may be inside, outside, or split.

        // Let `cut` represent the inside half-space of the cut, so `~cut` is
        // the outside half-space and `boundary(cut)` is the whole cut manifold.

        // (N-1)-dimensional polytopes that together comprise `boundary(polytope
        // ∩ cut)`, or equivalently, `boundary(polytope) ∩ cut`.
        let mut self_boundary_of_inside = AtomicPolytopeSet::new();

        // (N-1)-dimensional polytopes that together comprise `boundary(polytope
        // ∩ ~cut)`, or equivalently, `boundary(polytope) ∩ ~cut`.
        let mut self_boundary_of_outside = AtomicPolytopeSet::new();

        // (N-2)-dimensional polytopes that together comprise `boundary(polytope
        // ∩ boundary(cut))`, or equivalently, `boundary(polytope) ∩
        // boundary(cut)`.
        let mut intersection_boundary = AtomicPolytopeSet::new();

        // Split each of the "child" polytopes that comprise
        // `boundary(polytope)`.
        for &child in &polytope_boundary {
            match self.cut_atomic_polytope(child, cut)? {
                AtomicPolytopeCutOutput::Flush => {
                    bail!("manifold is flush, but has different ID")
                }
                AtomicPolytopeCutOutput::ManifoldInside => {
                    self_boundary_of_inside.insert(child);
                }
                AtomicPolytopeCutOutput::ManifoldOutside => {
                    self_boundary_of_outside.insert(child);
                }
                AtomicPolytopeCutOutput::NonFlush {
                    inside,
                    outside,
                    intersection,
                } => {
                    self_boundary_of_inside.extend(inside);
                    self_boundary_of_outside.extend(outside);
                    intersection_boundary.extend(intersection.map(|s| -s));
                }
            }
        }

        use crate::util::display_list;
        tracing::trace!(self_boundary_of_inside = %display_list(self_boundary_of_inside.iter()));
        tracing::trace!(self_boundary_of_outside = %display_list(self_boundary_of_outside.iter()));
        tracing::trace!(intersection_boundary = %display_list(intersection_boundary.iter()));

        // Simplify boundary of intersection.
        intersection_boundary =
            self.simplify_polytope_boundary(intersection_manifold, intersection_boundary)?;

        // Let `intersection` be the (N-1)-dimensional polytope that is
        // `polytope ∩ boundary(cut)`.
        let mut intersection = None;
        // There are two cases in which `intersection` should be nonempty:
        // - `intersection_boundary` is nonempty, so `intersection` should
        //   obviously be nonempty.
        // - The polytope completely contains the manifold, so `intersection`
        //   should be the entirety of `intersection_manifold` with no boundary.
        if !intersection_boundary.is_empty()
            || self.polytope_completely_contains_manifold(polytope, intersection_manifold.id)?
        {
            let new_polytope =
                self.add_atomic_polytope(intersection_manifold, intersection_boundary)?;

            // `polytope ∩ boundary(cut)` is part of the boundary of `polytope ∩
            // cut` and part of the boundary of `polytope ∩ ~cut`.
            self_boundary_of_inside.insert(new_polytope);
            self_boundary_of_outside.insert(-new_polytope);

            intersection = Some(new_polytope);
        }

        // Construct the N-dimensional polytope that is `self ∩ cut`
        let mut inside = None;
        if cut.params.inside == PolytopeFate::Keep && !self_boundary_of_inside.is_empty() {
            let s = tracing::info_span!("constructing inside polytope")
                .in_scope(|| self.add_atomic_subpolytope(polytope, self_boundary_of_inside))?;
            inside = Some(s);
        }

        // Construct the N-dimensional polytope that is `self ∩ ~cut`
        let mut outside = None;
        if cut.params.outside == PolytopeFate::Keep && !self_boundary_of_outside.is_empty() {
            let s = tracing::info_span!("constructing outside polytope")
                .in_scope(|| self.add_atomic_subpolytope(polytope, self_boundary_of_outside))?;
            outside = Some(s);
        };

        Ok(AtomicPolytopeCutOutput::NonFlush {
            inside: inside.map(AtomicPolytopeRef::from),
            outside: outside.map(AtomicPolytopeRef::from),
            intersection,
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

    fn simplify_polytope_boundary(
        &mut self,
        manifold: ManifoldRef,
        boundary: AtomicPolytopeSet,
    ) -> Result<AtomicPolytopeSet> {
        // This method is slightly questionable, since its return value doesn't
        // indicate the case where simplifying the boundary revealed that the
        // polytope cannot exist. It's not an issue in practice because it's
        // only ever called in a context where we can check through other means
        // whether the polytope should exist, even if it has no boundary.
        if self[manifold.id].ndim == 1 {
            Ok(self
                .simplify_intersection_of_intervals(manifold, boundary)
                .context("error simplifying boundary of 1D intersection")?
                .unwrap_or_else(AtomicPolytopeSet::new))
        } else {
            // Just remove duplicates (which `Set64` does automatically for us)
            // and cancel opposite signs.
            Ok(boundary
                .iter()
                .filter(|&elem| !boundary.contains(-elem))
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
        intervals: AtomicPolytopeSet,
    ) -> Result<Option<AtomicPolytopeSet>> {
        let mut simplified = AtomicPolytopeSet::new();
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
        existing_intervals: impl IntoIterator<Item = AtomicPolytopeRef>,
        mut new_interval: AtomicPolytopeRef,
    ) -> Result<Option<AtomicPolytopeSet>> {
        let mut simplified = AtomicPolytopeSet::new();
        for existing_interval in existing_intervals {
            // The intersection of intervals is the complement of the union of
            // the complements. (Negating a point pair corresponds to taking the
            // complement of an interval.)
            match self.try_union_intervals(space, -existing_interval, -new_interval)? {
                IntervalUnion::Union(union) => new_interval = -union,

                IntervalUnion::WholeSpace => return Ok(None), // whole space is excluded; there's nothing left

                IntervalUnion::Disconnected => {
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
        interval1: AtomicPolytopeRef,
        interval2: AtomicPolytopeRef,
    ) -> Result<IntervalUnion> {
        let [a, b] = self.extract_point_pair(interval1)?;
        let [p, q] = self.extract_point_pair(interval2)?;
        let ab = self.manifold_of(interval1);
        let pq = self.manifold_of(interval2);

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
                return Ok(IntervalUnion::WholeSpace);
            }

            start = if ab_has_p {
                &a
            } else if pq_has_a {
                &p
            } else {
                return Ok(IntervalUnion::Disconnected);
            };

            end = if ab_has_q {
                &b
            } else if pq_has_b {
                &q
            } else {
                return Ok(IntervalUnion::Disconnected);
            };
        }

        let new_point_pair_manifold =
            self.add_manifold(start.to_normalized_1blade() ^ end.to_normalized_1blade())?;
        let new_point_pair = self.add_point_pair(new_point_pair_manifold)?;
        Ok(IntervalUnion::Union(new_point_pair))
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
    /// manifold of `polytope`) is completely inside `polytope`.
    ///
    /// To be considered "completely inside," `manifold` may only touch the
    /// boundary of `polytope` at finitely many points. In other words, it can be
    /// tangent to the boundary of `polytope` but not flush with a boundary
    /// element.
    fn polytope_completely_contains_manifold(
        &self,
        polytope: AtomicPolytopeId,
        manifold: ManifoldId,
    ) -> Result<bool> {
        for boundary_elem in self.boundary_of(polytope) {
            match self.which_side(
                self.manifold_of(polytope),
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

    /// Returns whether the inside or outside of `cut` contains `point`, within
    /// `space`.
    pub fn which_side_has_point(
        &self,
        space: ManifoldRef,
        cut: ManifoldRef,
        point: &Point,
    ) -> PointWhichSide {
        self[cut.id]
            .blade
            .opns_to_ipns_in_space(&self[space.id].blade)
            .ipns_query_point(point)
            * (space.sign * cut.sign)
    }

    /// Outputs a multiline string representation of a polytope for debugging.
    pub fn polytope_to_string(&self, polytope: AtomicPolytopeRef) -> String {
        let mut buffer = String::new();
        self.polytope_to_string_internal(&mut buffer, polytope, 0);
        buffer
    }
    fn polytope_to_string_internal(
        &self,
        buffer: &mut String,
        polytope: AtomicPolytopeRef,
        indent: u8,
    ) {
        for _ in 0..indent {
            *buffer += "  ";
        }
        *buffer += &format!("{}#{:<5}", polytope.sign, polytope.id.0);
        let manifold = self.manifold_of(polytope);
        let blade = &self[manifold.id].blade * manifold.sign;
        if self[manifold.id].ndim == 0 {
            let [a, b] = blade.point_pair_to_points().expect("bad point pair");
            *buffer += &format!("{a}..{b}");
        } else {
            *buffer += &blade.to_string();
        }
        buffer.push('\n');
        for child in self.boundary_of(polytope) {
            self.polytope_to_string_internal(buffer, child, indent + 1);
        }
    }
}

/// Trait unifying polytopes and manifolds.
pub trait HasManifoldInSpace {
    /// Returns the manifold of `self` in `space`. If `self` is unsigned, it is
    /// assumed to have positive sign.
    fn get_manifold_ref(&self, space: &Space) -> ManifoldRef;
}
impl<T: HasManifoldInSpace> HasManifoldInSpace for &T {
    fn get_manifold_ref(&self, space: &Space) -> ManifoldRef {
        (*self).get_manifold_ref(space)
    }
}
impl HasManifoldInSpace for ManifoldId {
    fn get_manifold_ref(&self, _space: &Space) -> ManifoldRef {
        self.into()
    }
}
impl HasManifoldInSpace for ManifoldRef {
    fn get_manifold_ref(&self, _space: &Space) -> ManifoldRef {
        *self
    }
}
impl HasManifoldInSpace for AtomicPolytopeId {
    fn get_manifold_ref(&self, space: &Space) -> ManifoldRef {
        space[*self].manifold.into()
    }
}
impl HasManifoldInSpace for AtomicPolytopeRef {
    fn get_manifold_ref(&self, space: &Space) -> ManifoldRef {
        space[self.id].manifold * self.sign
    }
}
impl HasManifoldInSpace for AtomicPolytope {
    fn get_manifold_ref(&self, _space: &Space) -> ManifoldRef {
        self.manifold.into()
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
