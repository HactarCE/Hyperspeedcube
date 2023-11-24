//! TODO:
//! - document this
//! - do we really need `ConvexPolytope`, or can we just have `Polytope` consist
//!   of a bunch of `AtomicPolytopeRef`s?

use eyre::{bail, ensure, eyre, Result};
use itertools::Itertools;
use std::{
    collections::{hash_map, HashMap, VecDeque},
    ops::{Index, Neg},
};

mod cut;
mod patch;

use cut::{
    AtomicPatchCutId, AtomicPatchCutParams, PerAtomicPatchCut, PerAtomicPatchCutRegion,
    PortalCutInteraction,
};
pub use cut::{Cut, CutParams, PerGlobalCutRegion};

use super::*;
use hypermath::{
    collections::{generic_vec::IndexOutOfRange, GenericVec},
    prelude::*,
};
pub use patch::{PatchData, PatchGraph, Portal};

hypermath::idx_struct! {
    /// ID for a patch of a [`Space`].
    pub struct PatchId(pub u16);
    /// ID for a portal within a patch.
    pub struct PortalId(pub u8);
}

/// List containing a value per patch.
pub type PerPatch<T> = GenericVec<PatchId, T>;
/// List containing a value per portal.
pub type PerPortal<T> = GenericVec<PortalId, T>;

#[derive(Debug)]
pub struct PatchworkSpace {
    /// Base space.
    space: Space,
    /// Patch graph.
    patches: PatchGraph,
}
impl Index<PatchId> for PatchworkSpace {
    type Output = PatchData;

    fn index(&self, index: PatchId) -> &Self::Output {
        &self.patches[index]
    }
}
impl PatchworkSpace {
    pub fn new(ndim: u8) -> Result<Self> {
        let space = Space::new(ndim)?;
        let patches = PatchGraph::new(space.whole_space())?;
        Ok(PatchworkSpace { space, patches })
    }

    pub fn ndim(&self) -> u8 {
        self.space.ndim()
    }

    pub fn add_schlafli_patch(&mut self, schlafli: SchlafliSymbol) -> Result<PatchId> {
        ensure!(schlafli.ndim() <= self.space.ndim(), "bad schlafli symbol");

        let mut patch_polytope = self.space.whole_space();
        let mut portal_polytopes: PerPortal<AtomicPolytopeRef> = PerPortal::new();
        let mirrors = schlafli.mirrors();
        for Mirror(v) in &mirrors {
            let mut cut = AtomicCut::carve(self.space.add_plane(v, 0.0)?);

            // Cut existing portals.
            self.space.add_plane(v, 0.0)?;
            for (_, portal_polytope) in &mut portal_polytopes {
                *portal_polytope = self
                    .cut_atomic(*portal_polytope, &mut cut)?
                    .inside
                    .ok_or_else(|| eyre!("error cutting mirror by new mirror"))?;
            }

            // Cut patch.
            let patch_cut_output = self.cut_atomic(patch_polytope, &mut cut)?;
            patch_polytope = patch_cut_output
                .inside
                .ok_or_else(|| eyre!("error cutting patch by mirror"))?;

            // Add the new portal.
            portal_polytopes.push(
                patch_cut_output
                    .intersection
                    .ok_or_else(|| eyre!("error constructing new mirror"))?,
            )?;
        }

        let patch = self.patches.add_schlafli_patch(patch_polytope, schlafli)?;

        for (Mirror(v), (portal, portal_polytope)) in std::iter::zip(mirrors, portal_polytopes) {
            self.patches[patch].portals.push(Portal {
                isometry: Isometry::from_reflection(v).ok_or_else(|| eyre!("bad mirror vector"))?,
                other_patch: patch,
                portal_of_other_patch: portal,
                polytope: portal_polytope,
            })?;
        }

        Ok(patch)
    }

    fn cut_atomic(
        &mut self,
        polytope: AtomicPolytopeRef,
        cut: &mut AtomicCut,
    ) -> Result<CutOutput<AtomicPolytopeRef>> {
        let mut ret = CutOutput::default();
        match self.space.cut_atomic_polytope(polytope, cut)? {
            AtomicPolytopeCutOutput::Flush => bail!("polytope is flush with cut"),
            AtomicPolytopeCutOutput::ManifoldInside => ret.inside = Some(polytope),
            AtomicPolytopeCutOutput::ManifoldOutside => ret.outside = Some(polytope),
            AtomicPolytopeCutOutput::NonFlush {
                inside,
                outside,
                intersection,
                ..
            } => {
                ret.inside = inside;
                ret.outside = outside;
                ret.intersection = intersection;
            }
        }
        Ok(ret)
    }

    pub fn initial_patch(&self) -> PatchId {
        PatchId(0)
    }
    pub fn internal_space(&mut self) -> &mut Space {
        &mut self.space
    }
    pub fn add_manifold(&mut self, blade: Blade) -> Result<ManifoldRef> {
        self.space.add_manifold(blade)
    }

    pub fn carve(&mut self, initial_patch: PatchId, initial_manifold: ManifoldRef) -> Result<Cut> {
        let params = AtomicCutParams {
            divider: initial_manifold,
            inside: PolytopeFate::Keep,
            outside: PolytopeFate::Remove,
        };
        let params = self.expand_cut_params_thru_patches(initial_patch, params)?;
        Cut::new(self, params)
    }
    pub fn slice(&mut self, initial_patch: PatchId, initial_manifold: ManifoldRef) -> Result<Cut> {
        let params = AtomicCutParams {
            divider: initial_manifold,
            inside: PolytopeFate::Keep,
            outside: PolytopeFate::Keep,
        };
        let params = self.expand_cut_params_thru_patches(initial_patch, params)?;
        Cut::new(self, params)
    }

    /// Cuts a polytope and returns the resulting polytope (or `None`) for every
    /// global cut region.
    #[tracing::instrument(skip_all, fields(?cut, ?polytope))]
    pub fn cut(
        &mut self,
        cut: &mut Cut,
        polytope: &Polytope,
    ) -> Result<PerGlobalCutRegion<Option<Polytope>>> {
        let mut results = cut.regions.map_ref(|_, _| Polytope::default());

        // Break the polytope into convex components and cut each of those.
        for component in &polytope.components {
            for (global_cut_region, new_component) in self.cut_convex_polytope(cut, component)? {
                results[global_cut_region].components.push(new_component);
            }
        }

        // Remove empty polytopes.
        Ok(results.map(|_, mut result| {
            result.components.retain(|component| !component.is_empty());
            (!result.is_empty()).then_some(result)
        }))
    }

    #[tracing::instrument(skip_all, fields(?cut, ?polytope))]
    fn cut_convex_polytope(
        &mut self,
        cut: &mut Cut,
        polytope: &ConvexPolytope,
    ) -> Result<PerGlobalCutRegion<ConvexPolytope>> {
        // Break each convex component into atomic polytopes, which each exist
        // only in a single patch, and cut each of those.
        let cut_results: HashMap<PatchId, PerAtomicPatchCutRegion<Option<AtomicPolytopeRef>>> =
            polytope
                .by_patch
                .iter()
                .map(|(&patch, &atomic_polytope)| {
                    let patch_cut = cut
                        .by_patch
                        .get_mut(&patch)
                        .ok_or_else(|| eyre!("missing patch cut in patch {patch}"))?;
                    let cut_result = self.patch_cut_atomic_polytope(patch_cut, atomic_polytope)?;
                    Ok((patch, cut_result))
                })
                .collect::<Result<_>>()?;

        // Combine those atomic polytopes back into convex polytopes.
        Ok(cut.regions.map_ref(
            |_global_region_id, atomic_patch_cut_regions| ConvexPolytope {
                by_patch: atomic_patch_cut_regions
                    .iter()
                    .filter_map(|&(patch_id, atomic_patch_cut_region_id)| {
                        let atomic_polytope = cut_results[&patch_id][atomic_patch_cut_region_id]?;
                        Some((patch_id, atomic_polytope))
                    })
                    .collect(),
            },
        ))
    }

    /// Cuts an atomic polytope within a region and returns the resulting
    /// polytope (or `None`) for every atomic cut region.
    #[tracing::instrument(skip_all, fields(?patch_cut, ?polytope))]
    fn patch_cut_atomic_polytope(
        &mut self,
        patch_cut: &mut PerAtomicPatchCut<AtomicCut>,
        polytope: AtomicPolytopeRef,
    ) -> Result<PerAtomicPatchCutRegion<Option<AtomicPolytopeRef>>> {
        let mut atomic_polytope_results: Vec<Option<AtomicPolytopeRef>> = vec![Some(polytope)];

        // Cut the atomic polytope by each of `n` cuts, generating up to
        // `2^n` new atomic polytopes.
        for atomic_cut in patch_cut.iter_values_mut().rev() {
            atomic_polytope_results = atomic_polytope_results
                .into_iter()
                .map(|atomic_polytope_result| match atomic_polytope_result {
                    None => Ok([None, None]),
                    Some(atomic_polytope_result) => {
                        self.atomic_cut_atomic_polytope(atomic_cut, atomic_polytope_result)
                    }
                })
                .flatten_ok()
                .try_collect()?;
        }

        if 1 << patch_cut.len() != atomic_polytope_results.len() {
            bail!("mismatch between number of regions and number of atomic polytopes");
        }

        Ok(PerAtomicPatchCutRegion::from_iter(atomic_polytope_results))
    }

    /// Returns a pair of new `AtomicPolytopePatchCutOutput`s: `[outside,
    /// inside]`.
    #[tracing::instrument(skip_all, fields(?atomic_cut, ?existing))]
    fn atomic_cut_atomic_polytope(
        &mut self,
        atomic_cut: &mut AtomicCut,
        existing: AtomicPolytopeRef,
    ) -> Result<[Option<AtomicPolytopeRef>; 2]> {
        let mut outside_output = None;
        let mut inside_output = None;

        match self.space.cut_atomic_polytope(existing, atomic_cut)? {
            AtomicPolytopeCutOutput::Flush => {
                bail!("top-level polytope is flush with cut")
            }
            AtomicPolytopeCutOutput::ManifoldInside => inside_output = Some(existing),
            AtomicPolytopeCutOutput::ManifoldOutside => outside_output = Some(existing),
            AtomicPolytopeCutOutput::NonFlush {
                inside, outside, ..
            } => {
                inside_output = inside;
                outside_output = outside;
            }
        }

        Ok([outside_output, inside_output])
    }

    fn simple_atomic_cut_atomic_polytope_list(
        &mut self,
        atomic_cut: &mut AtomicCut,
        polytopes: &[AtomicPolytopeRef],
    ) -> Result<[Vec<AtomicPolytopeRef>; 2]> {
        let mut ret_outside = vec![];
        let mut ret_inside = vec![];
        for &p in polytopes {
            let [o, i] = self.simple_atomic_cut_atomic_polytope(atomic_cut, p)?;
            ret_outside.extend(o);
            ret_inside.extend(i);
        }
        Ok([ret_outside, ret_inside])
    }

    fn simple_atomic_cut_atomic_polytope(
        &mut self,
        atomic_cut: &mut AtomicCut,
        polytope: AtomicPolytopeRef,
    ) -> Result<[Option<AtomicPolytopeRef>; 2]> {
        match self.space.cut_atomic_polytope(polytope, atomic_cut)? {
            AtomicPolytopeCutOutput::Flush => Ok([Some(polytope), Some(polytope)]),
            AtomicPolytopeCutOutput::ManifoldInside => Ok([None, Some(polytope)]),
            AtomicPolytopeCutOutput::ManifoldOutside => Ok([Some(polytope), None]),
            AtomicPolytopeCutOutput::NonFlush {
                inside, outside, ..
            } => Ok([outside, inside]),
        }
    }

    #[tracing::instrument(skip(self))]
    fn add_patch_cut(
        &self,
        multicut: &mut CutParams,
        patch: PatchId,
        params: AtomicCutParams,
    ) -> Result<AtomicPatchCutId, IndexOutOfRange> {
        multicut[patch].atomic_cuts.push(AtomicPatchCutParams::new(
            params,
            self.patches[patch].portals.len(),
        ))
    }
    #[tracing::instrument(skip(self))]
    fn expand_cut_params_thru_patches(
        &mut self,
        initial_patch: PatchId,
        initial_cut_params: AtomicCutParams,
    ) -> Result<CutParams> {
        let mut cut_params = cut::CutParams::new();
        let initial_cut = self.add_patch_cut(&mut cut_params, initial_patch, initial_cut_params)?;

        let mut seen = HashMap::<(PatchId, ManifoldRef), AtomicPatchCutId>::new();
        seen.insert((initial_patch, initial_cut_params.divider), initial_cut);

        let mut queue = VecDeque::<(PatchId, AtomicPatchCutId)>::new();
        queue.push_back((initial_patch, initial_cut));

        while let Some((patch_id, patch_cut_id)) = queue.pop_front() {
            for (portal_id, portal) in &self.patches[patch_id].portals {
                let patch_cut_params = &mut cut_params[patch_id][patch_cut_id];

                if patch_cut_params[portal_id].is_some() {
                    continue; // Already computed!
                }

                let patch_cut_manifold = patch_cut_params.cut.divider;
                let which_side_has_portal = self
                    .space
                    .which_side_has_polytope(patch_cut_manifold, portal.polytope)?;

                match which_side_has_portal {
                    WhichSide::Inside { is_touching: false } => {
                        patch_cut_params.portal_interactions[portal_id] =
                            Some(PortalCutInteraction::Inside);
                    }
                    WhichSide::Outside { is_touching: false } => {
                        patch_cut_params.portal_interactions[portal_id] =
                            Some(PortalCutInteraction::Outside);
                    }
                    _ => {
                        ensure!(which_side_has_portal.is_touching(), "unreachable");

                        // The cut touches the portal. Create the corresponding
                        // cut on the other side.
                        let new_patch_id = portal.other_patch;
                        let untransformed_blade = self.space.blade_of(patch_cut_manifold);
                        let transformed_blade =
                            portal.isometry.transform_blade(&untransformed_blade);
                        let transformed_cut_manifold =
                            self.space.add_manifold(transformed_blade)?;
                        self.space.ndim_of(transformed_cut_manifold);

                        let new = (new_patch_id, transformed_cut_manifold);

                        let new_patch_cut_id: AtomicPatchCutId;
                        match seen.entry(new) {
                            hash_map::Entry::Occupied(e) => new_patch_cut_id = *e.get(),
                            hash_map::Entry::Vacant(e) => {
                                let new_cut_params = AtomicCutParams {
                                    divider: transformed_cut_manifold,
                                    ..initial_cut_params
                                };
                                new_patch_cut_id = self.add_patch_cut(
                                    &mut cut_params,
                                    new_patch_id,
                                    new_cut_params,
                                )?;
                                e.insert(new_patch_cut_id);
                                queue.push_back((new_patch_id, new_patch_cut_id));
                            }
                        };

                        // Link the cuts on either side of the portal.
                        cut_params[patch_id][patch_cut_id][portal_id] =
                            Some(PortalCutInteraction::Extension(new_patch_cut_id));
                        cut_params[new_patch_id][new_patch_cut_id][portal.portal_of_other_patch] =
                            Some(PortalCutInteraction::Extension(patch_cut_id));
                    }
                };
                ensure!(
                    seen.len() <= crate::MAX_PORTAL_EXPANSION,
                    "portal expansion exceeded maximum of {}",
                    crate::MAX_PORTAL_EXPANSION,
                )
            }
        }

        // Check that all cut-portal interactions are known.
        for patch_cut in cut_params.by_patch.values() {
            for cut in patch_cut.atomic_cuts.iter_values() {
                for portal_interaction in cut.portal_interactions.iter_values() {
                    if portal_interaction.is_none() {
                        bail!("missing portal interaction for cut");
                    }
                }
            }
        }

        Ok(cut_params)
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct CutOutput<P> {
    inside: Option<P>,
    outside: Option<P>,
    intersection: Option<P>,
}
impl<P> Default for CutOutput<P> {
    fn default() -> Self {
        Self {
            inside: None,
            outside: None,
            intersection: None,
        }
    }
}

/// Finite union of conformally convex polytopes.
#[derive(Debug, Default, Clone)]
pub struct Polytope {
    pub components: Vec<ConvexPolytope>,
}
impl<T: Into<ConvexPolytope>> From<T> for Polytope {
    fn from(value: T) -> Self {
        Polytope {
            components: vec![value.into()],
        }
    }
}
impl Polytope {
    pub fn is_empty(&self) -> bool {
        self.components.is_empty()
    }

    /// TODO: this is bad and should not exist. handle the general case of
    /// nonconvex/nonatomic polytopes!
    pub fn into_single_atomic_polytope(&self) -> Result<AtomicPolytopeRef> {
        (|| {
            let components = self.components.iter().exactly_one().ok()?;
            components.by_patch.values().exactly_one().ok().copied()
        })()
        .ok_or_else(|| eyre!("expected single atomic polytope"))
    }
}
impl Neg for &Polytope {
    type Output = Polytope;

    fn neg(self) -> Self::Output {
        Polytope {
            components: self.components.iter().map(|polytope| -polytope).collect(),
        }
    }
}

/// Conformally convex polytope which may span multiple patches.
#[derive(Debug, Default, Clone)]
pub struct ConvexPolytope {
    pub by_patch: HashMap<PatchId, AtomicPolytopeRef>,
}
impl ConvexPolytope {
    pub fn is_empty(&self) -> bool {
        self.by_patch.is_empty()
    }
}
impl Neg for &ConvexPolytope {
    type Output = ConvexPolytope;

    fn neg(self) -> Self::Output {
        ConvexPolytope {
            by_patch: self
                .by_patch
                .iter()
                .map(|(&patch, &polytope)| (patch, -polytope))
                .collect(),
        }
    }
}
impl From<(PatchId, AtomicPolytopeRef)> for ConvexPolytope {
    fn from(value: (PatchId, AtomicPolytopeRef)) -> Self {
        ConvexPolytope {
            by_patch: HashMap::from_iter([value]),
        }
    }
}
