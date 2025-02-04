//! Infinite Euclidean space in which polytopes can be constructed.
//!
//! In this module:
//! - A 0-dimensional **manifold** is always a pair of points.
//! - An N-dimensional **manifold** where N>0 is always closed (compact and with
//!   no boundary). More specifically, it is a hyperplane or hypersphere,
//!   represented using an OPNS blade in the [conformal geometric algebra].
//! - The **inside** and **outside** of a manifold are the half-spaces enclosed
//!   by it when embedded with an orientation into another manifold with one
//!   more dimension. In conformal geometry, the inside and outside must be
//!   determined by the orientation of the manifold rather than which half-space
//!   is finite.
//! - An **atomic polytope** in N-dimensional space is the intersection of the
//!   **inside**s of finitely many (N-1)-dimensional manifolds. It is
//!   represented as an N-dimensional manifold (on which the polytope lives) and
//!   a set of oriented (N-1)-dimensional polytopes that bound it.
//!
//! [conformal geometric algebra]: https://w.wiki/7SP3
//!
//! Atomic polytopes are memoized and given IDs.

use std::cmp::Ordering;
use std::collections::{hash_map, HashMap, HashSet};
use std::fmt;
use std::ops::{Index, Mul, MulAssign, Neg};

use eyre::{bail, ensure, eyre, Context, OptionExt, Result};
use float_ord::FloatOrd;
use hypermath::cga::*;
use hypermath::prelude::*;
use itertools::Itertools;
use tinyset::Set64;

mod atomic_polytope;
mod cut;
mod manifold;
mod map;
mod results;
mod signedref;

pub use atomic_polytope::AtomicPolytope;
pub use cut::{AtomicCut, AtomicCutParams, PolytopeFate};
pub use manifold::ManifoldData;
pub use map::{SpaceMap, SpaceMapFor};
use results::IntervalUnion;
pub use results::{AtomicPolytopeCutOutput, WhichSide};
pub use signedref::SignedRef;

use crate::SlabMap;

/// Reference to an oriented manifold in a [`Space`].
pub type ManifoldRef = SignedRef<ManifoldId>;
/// Reference to an oriented atomic polytope in a [`Space`].
pub type AtomicPolytopeRef = SignedRef<AtomicPolytopeId>;

/// Set of oriented manifolds in a [`Space`].
pub type ManifoldSet = Set64<ManifoldRef>;
/// Set of oriented atomic polytopes in a [`Space`].
pub type AtomicPolytopeSet = Set64<AtomicPolytopeRef>;
/// Set of unoriented atomic polytopes in a [`Space`].
pub type AtomicPolytopeIdSet = Set64<AtomicPolytopeId>;

hypermath::idx_struct! {
    /// ID for a memoized unoriented manifold in a [`Space`].
    pub struct ManifoldId(pub u32);
    /// ID for a memoized unoriented atomic polytope in a [`Space`].
    pub struct AtomicPolytopeId(pub u32);
    /// ID for a memoized isometry in a [`Space`].
    pub struct IsometryId(pub u32);
}

/// Euclidean space in which polytopes can be constructed.
pub struct Space {
    /// Submanifolds of the space.
    manifolds: SlabMap<ManifoldId, ManifoldData>,
    /// Atomic polytopes defined in the space.
    polytopes: SlabMap<AtomicPolytopeId, AtomicPolytope>,
    /// Isometries defined in the space.
    isometries: SlabMap<IsometryId, Isometry>,

    /// Manifold of the entire space.
    covering_manifold: ManifoldId,
    /// Polytope with no border covering the entire space.
    covering_polytope: AtomicPolytopeId,

    /// Cache for reverse of an isometry.
    transform_reverse_cache: HashMap<IsometryId, IsometryId>,
    /// Cache for composition of two isometries.
    transform_composition_cache: HashMap<(IsometryId, IsometryId), IsometryId>,
    /// Cache for transformation of a manifold.
    manifold_transform_cache: HashMap<(IsometryId, ManifoldId), ManifoldRef>,
    /// Cache for polytope which-side checks. This is not used by default (as
    /// most cuts are only performed once).
    polytope_which_side_cache: HashMap<(ManifoldId, AtomicPolytopeId), WhichSide>,
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

impl Index<IsometryId> for Space {
    type Output = Isometry;

    fn index(&self, index: IsometryId) -> &Self::Output {
        &self.isometries[index]
    }
}

impl Space {
    /// Constructs a new Euclidean space.
    pub fn new(ndim: u8) -> Result<Self> {
        let mut manifolds = SlabMap::new();
        let mut polytopes = SlabMap::new();
        let transforms = SlabMap::new();

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
            isometries: transforms,

            covering_manifold,
            covering_polytope,

            transform_reverse_cache: HashMap::new(),
            transform_composition_cache: HashMap::new(),
            manifold_transform_cache: HashMap::new(),
            polytope_which_side_cache: HashMap::new(),
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

    /// Returns all children of `root` that have the given number of dimensions.
    ///
    /// Polytopes are signed, so the same polytope may be returned twice with
    /// different signs; otherwise, there are no duplicates.
    #[allow(clippy::comparison_chain)]
    pub fn children_with_ndim(&self, root: AtomicPolytopeRef, ndim: u8) -> Vec<AtomicPolytopeRef> {
        let mut queue = vec![root];
        let mut seen = HashSet::new();
        let mut results = vec![];

        while let Some(shape) = queue.pop() {
            let shape_ndim = self.ndim_of(shape);
            if shape_ndim == ndim {
                results.push(shape);
            } else if shape_ndim > ndim {
                // TODO: handle non-flat shapes
                for b in self.boundary_of(shape) {
                    if seen.insert(b.id) {
                        queue.push(b);
                    }
                }
            }
        }

        results
    }

    /// Returns the pair of points that comprise a 0D polytope.
    pub fn extract_point_pair(&self, polytope: impl HasManifoldInSpace) -> Result<[Point; 2]> {
        self.blade_of(polytope)
            .point_pair_to_points()
            .ok_or_eyre("attempt to get point pair from non-point point pair manifold")
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
    #[tracing::instrument(skip(self), fields(%blade))]
    pub fn add_manifold(&mut self, blade: Blade) -> Result<ManifoldRef> {
        if blade.ndim() > self.ndim() {
            bail!("blade {blade} cannot fit in {}D space!", self.ndim());
        }

        // Canonicalize blade.
        let (blade, sign) = canonicalize_blade(blade)?;

        let manifold_data = ManifoldData::new(blade)?;
        if manifold_data.ndim > self.ndim() {
            bail!("manifold {manifold_data} does not fit inside space");
        }
        let manifold_id = self.manifolds.get_or_insert(manifold_data)?.key();

        Ok(manifold_id * sign)
    }
    /// Adds a set of manifolds to the space.
    pub fn add_manifolds(
        &mut self,
        blades: impl IntoIterator<Item = Blade>,
    ) -> Result<ManifoldSet> {
        blades.into_iter().map(|b| self.add_manifold(b)).collect()
    }

    /// Adds a point pair to the space.
    fn add_point_pair(&mut self, manifold: ManifoldRef) -> Result<AtomicPolytopeRef> {
        if self.ndim_of(manifold) != 0 {
            bail!("add_point_pair() requires ndim = 0");
        }

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
        if self.ndim_of(manifold) == 0 {
            ensure!(boundary.is_empty(), "point pair must have no boundary");
            return self.add_point_pair(manifold);
        }

        let unsigned_boundary = boundary.into_iter().map(|b| b * manifold.sign).collect();
        let polytope_data = AtomicPolytope::new(manifold.id, unsigned_boundary);
        let polytope_id = self.get_or_insert_polytope_data(polytope_data)?;
        Ok(polytope_id * manifold.sign)
    }
    /// Adds an atomic polytope using the manifold of an existing polytope, or
    /// reuses the existing polytope if possible. In particular, if
    /// `new_boundary` is the same as the boundary of `old_polytope`, then this
    /// method returns `old_polytope`; otherwise it creates and returns a new
    /// polytope.
    #[tracing::instrument(skip(self))]
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
    #[tracing::instrument(skip(self))]
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
            if ndim != self.ndim_of(boundary_elem.id) + 1 {
                bail!("polytope ndim does not match boundary ndim+1");
            }
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
                tracing::info!("{}", self.polytope_to_string(polytope.into()));
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

    /// Returns a set of the elements of a polytope, of all ranks.
    pub fn elements_of(&self, root: AtomicPolytopeId) -> Result<AtomicPolytopeIdSet> {
        let mut ret = AtomicPolytopeIdSet::new();
        let mut queue = vec![root];
        while let Some(p) = queue.pop() {
            if ret.insert(p) {
                for elem in self.boundary_of(p) {
                    queue.push(elem.id);
                }
            }
        }
        Ok(ret)
    }

    /// Adds an isometry to the space.
    pub fn add_isometry(&mut self, isometry: Isometry) -> Result<IsometryId> {
        Ok(self.isometries.get_or_insert(isometry)?.key())
    }

    /// Cuts each atomic polytope in a set.
    pub fn cut_atomic_polytope_set(
        &mut self,
        polytopes: AtomicPolytopeSet,
        cut: &mut AtomicCut,
    ) -> Result<AtomicPolytopeSet> {
        let mut ret = AtomicPolytopeSet::new();
        for polytope in polytopes {
            let mut inside = None;
            let mut outside = None;
            match self.cut_atomic_polytope(polytope, cut)? {
                AtomicPolytopeCutOutput::Flush => continue,
                AtomicPolytopeCutOutput::ManifoldInside => inside = Some(polytope),
                AtomicPolytopeCutOutput::ManifoldOutside => outside = Some(polytope),
                AtomicPolytopeCutOutput::NonFlush {
                    inside: i,
                    outside: o,
                    intersection: _,
                    is_intersection_new: _,
                } => {
                    inside = i;
                    outside = o;
                }
            }
            if cut.params.inside == PolytopeFate::Keep {
                ret.extend(inside);
            }
            if cut.params.outside == PolytopeFate::Keep {
                ret.extend(outside);
            }
        }
        Ok(ret)
    }

    /// Cuts an atomic polytope.
    #[tracing::instrument(skip(self), fields(%cut))]
    pub fn cut_atomic_polytope(
        &mut self,
        polytope: AtomicPolytopeRef,
        cut: &mut AtomicCut,
    ) -> Result<AtomicPolytopeCutOutput> {
        let cut_ndim = self.ndim_of(cut.params.divider);
        let space_ndim = self.ndim();
        ensure!(
            cut_ndim == space_ndim - 1,
            "expected {}D cut in {space_ndim}D space; got {cut_ndim}D cut",
            space_ndim - 1,
        );

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
    #[tracing::instrument(skip(self), fields(%cut), ret(Display), err(Debug))]
    fn cut_atomic_polytope_uncached(
        &mut self,
        polytope: AtomicPolytopeId,
        cut: &mut AtomicCut,
    ) -> Result<AtomicPolytopeCutOutput> {
        match cut.which_side_of_cut_has_manifold(self, self.manifold_of(polytope).id)? {
            WhichSide::Flush => Ok(AtomicPolytopeCutOutput::Flush),
            WhichSide::Inside { .. } => Ok(AtomicPolytopeCutOutput::ManifoldInside),
            WhichSide::Outside { .. } => Ok(AtomicPolytopeCutOutput::ManifoldOutside),
            WhichSide::Split => {
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
    #[tracing::instrument(skip(self), fields(%cut))]
    fn cut_atomic_polytope_1d(
        &mut self,
        polytope: AtomicPolytopeId,
        cut: &mut AtomicCut,
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
            is_intersection_new: true,
        })
    }

    /// Cuts an N-dimensional atomic polytope, assuming it is split by the cut.
    #[tracing::instrument(skip(self), fields(%cut))]
    fn cut_atomic_polytope_nd(
        &mut self,
        polytope: AtomicPolytopeId,
        cut: &mut AtomicCut,
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
                    is_intersection_new: false,
                });
            }
        }

        // Next, scan for any boundary polytope whose manifold is completely
        // contained inside the cut.
        for &child in &polytope_boundary {
            // Which side of the cut contains the child?
            let child_manifold = self.manifold_of(child).id;
            let which_side_of_cut_has_child =
                cut.which_side_of_cut_has_manifold(self, child_manifold)?;
            match which_side_of_cut_has_child {
                WhichSide::Flush => {
                    bail!("manifold is flush, but has different ID")
                }
                WhichSide::Split => continue,
                _ => (),
            }

            // Which side of the *child* contains the *cut*?
            let which_side_of_child_has_cut = self.which_side_has_manifold(
                self.manifold_of(polytope),
                self.manifold_of(child),
                intersection_manifold.id,
            )?;
            if matches!(which_side_of_child_has_cut, WhichSide::Outside { .. }) {
                tracing::debug!("found child {child} that excludes cut");

                // Based on just this child, the cut is completely outside
                // `polytope`. So we know that `polytope` is either completely
                // inside the cut or completely outside the cut.
                let mut inside = None;
                let mut outside = None;
                match which_side_of_cut_has_child {
                    WhichSide::Flush | WhichSide::Split => {
                        unreachable!("cases already handled")
                    }
                    WhichSide::Inside { .. } => inside = Some(AtomicPolytopeRef::from(polytope)),
                    WhichSide::Outside { .. } => outside = Some(AtomicPolytopeRef::from(polytope)),
                }
                return Ok(AtomicPolytopeCutOutput::NonFlush {
                    inside,
                    outside,
                    intersection: None,
                    is_intersection_new: false,
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
                    is_intersection_new: _,
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
        // - `intersection_boundary` is nonempty, so `intersection` should obviously be
        //   nonempty.
        // - The polytope completely contains the manifold, so `intersection` should be
        //   the entirety of `intersection_manifold` with no boundary.
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
            is_intersection_new: intersection.is_some(),
        })
    }

    /// Given N-dimensional `space` containing `target` and (N-1)-dimensional
    /// `cut`, returns whether `target` is at least partly contained by either
    /// half of `space` separated by `cut`.
    ///
    /// Which part of `space` is considered "inside" or "outside" depends on the
    /// orientations of `space` and `cut`. The orientation of `target` makes no
    /// difference.
    #[tracing::instrument(skip(self))]
    pub fn which_side_has_manifold(
        &self,
        space: ManifoldRef,
        cut: ManifoldRef,
        target: ManifoldId,
    ) -> Result<WhichSide> {
        let sign = space.sign * cut.sign;

        let space = &self[space.id];
        let cut = &self[cut.id];
        let target = &self[target];

        if space.ndim < 1 {
            bail!("which_side_has_manifold() was called with `space` lower than 2D")
        }

        if target.ndim == space.ndim {
            // `target` = `space`, and `cut` is a submanifold of `space`, so
            // `target` must be split.
            return Ok(WhichSide::Split);
        }
        // Otherwise `target` must be a submanifold of `space`.
        if target.ndim >= space.ndim {
            bail!("`target` is not a submanifold of `space`");
        }

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

            // 1. Compute the dual of the intersection of `target` and `cut`. I think this
            //    represents a bundle of all the manifolds that are perpendicular to
            //    `target` and `cut`.
            let perpendicular_bundle = &target_ipns ^ &cut_ipns;

            if perpendicular_bundle.is_zero() {
                return Ok(WhichSide::Flush);
            }

            // 2. Wedge with an arbitrary point to select one of those possible
            //    perpendicular manifolds. The only restriction here is that we don't want
            //    the wedge product to be zero.
            let perpendicular_manifold = nonzero_wedge_with_arbitrary_point(&perpendicular_bundle)?;

            // 3. Intersect that perpendicular manifold with `target` to get two points on
            //    `target`.
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
        Ok(
            WhichSide::from_points([cut_ipns.ipns_query_point(&a), cut_ipns.ipns_query_point(&b)])
                * sign,
        )
    }

    /// Given the N-dimensional `space` containing (N-1)-dimensional `cut` and
    /// M-dimensional `target` where M<=N, returns the (M-1)-dimensional
    /// intersection of `target` and `cut`. If `target` and `cut` do not
    /// intersect or if any of the other preconditions are broken, this function
    /// may return an error or garbage.
    ///
    /// The orientation of the result depends on the orientations of `space`,
    /// `cut`, and `target`.
    #[tracing::instrument(skip(self))]
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

        if cut.ndim + 1 != space.ndim {
            bail!("`cut` is not a (N-1)-dimensional submanifold of N-dimensional `space`");
        }
        if target.ndim > space.ndim {
            bail!("`target` is not a submanifold of `space`");
        }

        // Compute a "meet" which is the dual of the outer product.
        let intersection = Blade::meet_in_space(&cut.blade, &target.blade, &space.blade);
        if !intersection.opns_is_real() {
            bail!("intersection {intersection} is imaginary");
        }

        Ok(self.add_manifold(intersection)? * sign)
    }

    #[tracing::instrument(skip(self))]
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
                .wrap_err("error simplifying boundary of 1D intersection")?
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
    #[tracing::instrument(skip(self))]
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

                IntervalUnion::WholeSpace => return Ok(None), /* whole space is excluded; */
                // there's nothing left
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

        let new_point_pair_manifold = self.add_manifold(start.to_1blade() ^ end.to_1blade())?;
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
    /// boundary of `polytope` at finitely many points. In other words, it can
    /// be tangent to the boundary of `polytope` but not flush with a
    /// boundary element.
    #[tracing::instrument(skip(self))]
    fn polytope_completely_contains_manifold(
        &self,
        polytope: AtomicPolytopeId,
        manifold: ManifoldId,
    ) -> Result<bool> {
        for boundary_elem in self.boundary_of(polytope) {
            match self.which_side_has_manifold(
                self.manifold_of(polytope),
                self.manifold_of(boundary_elem),
                manifold,
            )? {
                WhichSide::Inside { .. } => continue,
                WhichSide::Flush | WhichSide::Outside { .. } | WhichSide::Split => {
                    return Ok(false)
                }
            }
        }
        Ok(true)
    }

    /// Returns whether the inside or outside of `cut` contains `point`, within
    /// `space`.
    #[tracing::instrument(skip(self))]
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

    /// Returns whether the inside or outside of (N-1)-dimensional `cut`
    /// contains `polytope`, where N is the number of dimensions of the whole
    /// space.
    #[tracing::instrument(skip(self))]
    fn which_side_has_polytope(
        &mut self,
        cut: ManifoldRef,
        polytope: AtomicPolytopeId,
        cache: bool,
    ) -> Result<WhichSide> {
        self.which_side_has_polytope_within_space(self.manifold(), cut, polytope, cache)
    }
    /// Returns whether the inside or outside of (N-1)-dimensional `cut`
    /// contains `polytope` in N-dimensional `space`.
    #[tracing::instrument(skip(self))]
    fn which_side_has_polytope_within_space(
        &mut self,
        space: ManifoldRef,
        cut: ManifoldRef,
        polytope: AtomicPolytopeId,
        cache: bool,
    ) -> Result<WhichSide> {
        if let Some(&result) = self.polytope_which_side_cache.get(&(cut.id, polytope)) {
            return Ok(result * cut.sign);
        }

        if self.ndim_of(cut) + 1 != self.ndim_of(space) {
            bail!("cut is not (N-1)-dimensional manifold in N-dimensional space");
        }

        let manifold_result =
            self.which_side_has_manifold(space, cut, self.manifold_of(polytope).id)?;

        let polytope_result = if self.boundary_of(polytope).next().is_none() {
            // If the polytope has no boundary (such as if it is a point pair)
            // then the manifold result is accurate.
            manifold_result
        } else {
            match manifold_result {
                // If the manifolds are flush, then the polytope is flush with
                // the cut. If the manifolds don't touch at all, then the
                // manifold result is accurate for the polytope.
                WhichSide::Flush | WhichSide::Inside | WhichSide::Outside => manifold_result,

                // If the manifolds intersect, then compute their intersection
                // and check whether that intersection is contained in the
                // polytope.
                WhichSide::Split => {
                    ensure!(self.ndim_of(polytope) > 0, "unreachable");
                    let intersection = self.intersect(space, cut, self.manifold_of(polytope))?;
                    self.which_side_has_polytope_within_polytope_manifold(
                        intersection,
                        polytope,
                        cache,
                    )?
                }
            }
        };

        if cache {
            self.polytope_which_side_cache
                .insert((cut.id, polytope), polytope_result * cut.sign);
        }

        Ok(polytope_result)
    }
    /// Returns whether the inside or outside of (N-1)-dimensional `cut`
    /// contains N-dimensional `polytope`.
    #[tracing::instrument(skip(self))]
    fn which_side_has_polytope_within_polytope_manifold(
        &mut self,
        cut: ManifoldRef,
        polytope: AtomicPolytopeId,
        cache: bool,
    ) -> Result<WhichSide> {
        if self.ndim_of(cut) + 1 != self.ndim_of(polytope) {
            bail!("cut is not one dimension lower than polytope");
        }
        if self.ndim_of(polytope) == 0 {
            return self.which_side_has_polytope_1d(cut, polytope);
        }

        let space = self.manifold_of(polytope);

        // Check whether `cut` is flush with any boundary element. If it is,
        // then the relative orientation indicates which side of the cut
        // contains the manifold.
        for boundary_elem in self.boundary_of(polytope) {
            let manifold_of_boundary_elem = self.manifold_of(boundary_elem);
            if cut.id == manifold_of_boundary_elem.id {
                return match cut.sign == manifold_of_boundary_elem.sign {
                    true => Ok(WhichSide::Inside),
                    false => Ok(WhichSide::Outside),
                };
            }
        }

        // Check whether `cut` is inside all boundary elements. If it is, then
        // it's inside the polytope.
        let mut is_inside_all = true;
        for boundary_elem in self.boundary_of(polytope) {
            let manifold_of_boundary_elem = self.manifold_of(boundary_elem);
            let which_side_has_cut =
                self.which_side_has_manifold(space, manifold_of_boundary_elem, cut.id)?;
            // Where is `cut` with respect to `boundary_elem`?
            match which_side_has_cut {
                // We should have already handled this case.
                WhichSide::Flush => bail!("manifolds are flush but not the same"),

                // This tells us nothing new.
                WhichSide::Inside { .. } => (),

                // If `cut` is outside one of the boundary elements of
                // `polytope`, then it's outside `polytope`.
                WhichSide::Outside => {
                    is_inside_all = false;
                }
                WhichSide::Split => {
                    // We don't actually know whether `cut` is touching the
                    // polytope. All we know is that `cut` is definitely not
                    // inside the polytope.
                    is_inside_all = false;
                }
            }
        }
        if is_inside_all {
            // If the manifold is inside the polytope, then it trivially splits
            // the polytope.
            return Ok(WhichSide::Split);
        }

        // Check whether any boundary element is inside `cut`, ...
        let mut is_any_inside = false;
        // ... or outside `cut`.
        let mut is_any_outside = false;
        for boundary_elem in self.boundary_of(polytope).collect_vec() {
            let which_side_has_polytope =
                self.which_side_has_polytope_within_space(space, cut, boundary_elem.id, cache)?;
            match which_side_has_polytope {
                WhichSide::Flush => bail!("unexpected flush polytope (case already handled)"),
                WhichSide::Inside { .. } => is_any_inside = true,
                WhichSide::Outside { .. } => is_any_outside = true,
                WhichSide::Split => return Ok(WhichSide::Split),
            }

            // Early return if any inside & any ouside.
            if is_any_inside && is_any_outside {
                return Ok(WhichSide::Split);
            }
        }
        // Now `is_any_inside` and `is_any_outside` are mutually exclusive.
        let boundary_is_all_inside = is_any_inside;
        let boundary_is_all_outside = is_any_outside;

        // We know that `cut` isn't inside the polytope, and it doesn't
        // intersect the boundary, so it must be entirely on the outside of the
        // polytope. The only questions is which side of it the polytope is on.
        if boundary_is_all_inside {
            Ok(WhichSide::Inside)
        } else if boundary_is_all_outside {
            Ok(WhichSide::Outside)
        } else {
            // Impossible! The first loop should've handled the case where there
            // are no boundary elements.
            bail!("impossible! boundaryless case should already be handled")
        }
    }

    /// Returns whether the inside or outside of point pair `cut` contains 1D
    /// `polytope`, within 1D `space`.
    fn which_side_has_polytope_1d(
        &mut self,
        cut: ManifoldRef,
        polytope: AtomicPolytopeId,
    ) -> Result<WhichSide> {
        let cut_polytope = self.add_point_pair(cut)?;

        let space = self.manifold_of(polytope);

        let intervals = self.boundary_of(polytope).collect_vec();
        let is_any_inside = self
            .incrementally_simplify_intersection_of_intervals(space, intervals, cut_polytope)?
            .is_some();

        let intervals = self.boundary_of(polytope).collect_vec();
        let is_any_outside = self
            .incrementally_simplify_intersection_of_intervals(space, intervals, -cut_polytope)?
            .is_some();

        match (is_any_inside, is_any_outside) {
            (true, true) => Ok(WhichSide::Split),
            (true, false) => Ok(WhichSide::Inside),
            (false, true) => Ok(WhichSide::Outside),
            (false, false) => Ok(WhichSide::Flush),
        }
    }

    /// Returns the location of `point` relative to `polytope`, assuming they
    /// are in the same manifold.
    pub fn is_polytope_touching_point(
        &mut self,
        point: &Point,
        polytope: AtomicPolytopeRef,
    ) -> Result<PointWhichSide> {
        let mut is_touching_any = false;
        for boundary_manifold in self.boundary_of(polytope) {
            match self.which_side_has_point(
                self.manifold_of(polytope),
                self.manifold_of(boundary_manifold),
                point,
            ) {
                PointWhichSide::On => is_touching_any = true,
                PointWhichSide::Inside => (),
                PointWhichSide::Outside => return Ok(PointWhichSide::Outside),
            }
        }
        if is_touching_any {
            Ok(PointWhichSide::On)
        } else {
            Ok(PointWhichSide::Inside)
        }
    }

    /// Composes two transforms `a * b`. Results are cached.
    pub fn compose_transforms(&mut self, a: IsometryId, b: IsometryId) -> Result<IsometryId> {
        let key = (a, b);
        if let Some(&result) = self.transform_composition_cache.get(&key) {
            Ok(result)
        } else {
            let result = self.add_isometry(&self[a] * &self[b])?;
            self.transform_composition_cache.insert(key, result);
            Ok(result)
        }
    }
    /// Reverses a transform. Results are cached.
    pub fn reverse_transform(&mut self, t: IsometryId) -> Result<IsometryId> {
        if let Some(&result) = self.transform_reverse_cache.get(&t) {
            Ok(result)
        } else {
            let result = self.add_isometry(self[t].reverse())?;
            self.transform_reverse_cache.insert(t, result);
            Ok(result)
        }
    }

    /// Transforms `manifold` by `transform`. Results are cached.
    pub fn transform_manifold(
        &mut self,
        isometry: IsometryId,
        manifold: ManifoldRef,
    ) -> Result<ManifoldRef> {
        let key = (isometry, manifold.id);
        if let Some(&result) = self.manifold_transform_cache.get(&key) {
            Ok(result * manifold.sign)
        } else {
            let blade = &self[manifold.id].blade;
            let result = self.add_manifold(self[isometry].transform_blade(blade))?;
            self.manifold_transform_cache.insert(key, result);
            Ok(result * manifold.sign)
        }
    }
    /// Returns which side of `manifold` contains `polytope`. Results are
    /// cached.
    pub fn cached_which_side_has_polytope(
        &mut self,
        manifold: ManifoldRef,
        polytope: AtomicPolytopeId,
    ) -> Result<WhichSide> {
        let key = (manifold.id, polytope);
        if let Some(&result) = self.polytope_which_side_cache.get(&key) {
            Ok(result * manifold.sign)
        } else {
            let result = self.which_side_has_polytope(manifold.id.into(), polytope, true)?;
            self.polytope_which_side_cache.insert(key, result);
            Ok(result * manifold.sign)
        }
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
    let first_term = blade
        .mv()
        .nonzero_terms()
        .next()
        .ok_or_eyre("zero manifold is not valid")?;
    let sign = Sign::from(first_term.coef);

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
        .ok_or_else(|| eyre!("unable to find point not on object {opns_blade}"))
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
            .map_err(|_| eyre!("error computing center of sphere"))?;
        Ok([
            Point::Finite(vector![radius] + &center),
            Point::Finite(vector![-radius] + &center),
        ])
    } else {
        Ok([Point::Finite(ipns.ipns_plane_pole()), Point::Infinity])
    }
}
