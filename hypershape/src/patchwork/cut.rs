use eyre::{bail, eyre, Result};
use std::{
    collections::{HashMap, HashSet, VecDeque},
    ops::{Index, IndexMut},
};

use hypermath::{collections::GenericVec, idx_struct};

use crate::{AtomicCut, AtomicCutParams, AtomicPolytopeRef, PatchworkSpace};

pub use super::{PatchGraph, PatchId, PerPortal, PortalId};

#[derive(Debug, Default)]
pub struct Cut {
    pub(super) regions: PerGlobalCutRegion<Vec<(PatchId, AtomicPatchCutRegionId)>>,
    pub(super) by_patch: HashMap<PatchId, PerAtomicPatchCut<AtomicCut>>,
}
impl Cut {
    pub fn new(space: &mut PatchworkSpace, params: CutParams) -> Result<Self> {
        let mut cut = Cut {
            regions: PerGlobalCutRegion::new(),
            by_patch: HashMap::new(),
        };
        dbg!(&params);

        let mut atomic_patch_cut_region_polytopes =
            HashMap::<(PatchId, AtomicPatchCutRegionId), AtomicPolytopeRef>::new();
        let mut unprocessed_atomic_patch_cut_regions =
            HashSet::<(PatchId, AtomicPatchCutRegionId)>::new();
        // For each patch ...
        for (&patch_id, patch_cut_params) in &params.by_patch {
            // Cut up the patch polytope to determine how many nonempty atomic
            // patch cut regions there are.
            let mut regions = vec![(AtomicPatchCutRegionId(0), space.patches[patch_id].polytope)];
            let mut atomic_cuts = PerAtomicPatchCut::new();
            for (atomic_cut_id, atomic_cut_params) in &patch_cut_params.atomic_cuts {
                let mut atomic_cut = AtomicCut::new(atomic_cut_params.cut);
                for (region_id, polytope) in std::mem::take(&mut regions) {
                    // Cut the region polytope.
                    let result = space.cut_atomic(polytope, &mut atomic_cut)?;
                    // Ignore empty regions.
                    if let Some(p) = result.inside {
                        regions.push((region_id.and_inside(atomic_cut_id), p));
                    }
                    if let Some(p) = result.outside {
                        regions.push((region_id.and_outside(atomic_cut_id), p));
                    }
                }
                atomic_cuts.push(atomic_cut)?;
            }
            cut.by_patch.insert(patch_id, atomic_cuts);
            atomic_patch_cut_region_polytopes
                .extend(regions.iter().map(|&(r, p)| ((patch_id, r), p)));
            unprocessed_atomic_patch_cut_regions.extend(regions.into_iter().map(
                |(atomic_patch_cut_region_id, _polytope)| (patch_id, atomic_patch_cut_region_id),
            ));
        }

        // Pop regions from the set one at a time, and expand them through all
        // the portals.
        println!(
            "Unproccessed atomic patch cut regions: {:?}",
            unprocessed_atomic_patch_cut_regions,
        );
        while let Some(&seed) = unprocessed_atomic_patch_cut_regions.iter().next() {
            // We've discovered a new global cut region!
            unprocessed_atomic_patch_cut_regions.remove(&seed);
            println!("Processing atomic patch cut region {seed:?}");

            let mut atomic_regions = HashSet::new();
            atomic_regions.insert(seed);
            dbg!(&atomic_regions);

            // Follow portals to find out all the other atomic cut regions in
            // this new global cut region.
            let mut queue: VecDeque<(PatchId, AtomicPatchCutRegionId)> = vec![seed].into();
            while let Some((patch_id, start_region)) = queue.pop_front() {
                for (portal_id, _portal) in &space.patches[patch_id].portals {
                    // If this atomic patch cut region has a matching region on
                    // the other side of the portal, figure out what its ID is.
                    if let Some(destination_region) = send_atomic_patch_cut_region_thru_portal(
                        &space.patches,
                        &params,
                        patch_id,
                        portal_id,
                        start_region,
                    )? {
                        // We now know the matching atomic cut region after passing
                        // through the portal!
                        if unprocessed_atomic_patch_cut_regions.remove(&destination_region) {
                            queue.push_back(destination_region);
                            atomic_regions.insert(destination_region);
                        } else if !atomic_regions.contains(&destination_region) {
                            println!("{:#?}", destination_region);
                            bail!("atomic region mismatch")
                        }
                    }
                }
            }

            cut.regions.push(atomic_regions.into_iter().collect())?;
        }

        Ok(cut)
    }
}

#[derive(Debug, Default)]
pub struct CutParams {
    pub(super) by_patch: HashMap<PatchId, PatchCutParams>,
}
impl CutParams {
    pub(super) fn new() -> Self {
        Self::default()
    }
    pub(super) fn in_patch(&mut self, patch: PatchId) -> &mut PatchCutParams {
        self.by_patch.entry(patch).or_default()
    }
}
impl Index<PatchId> for CutParams {
    type Output = PatchCutParams;

    fn index(&self, index: PatchId) -> &Self::Output {
        const DEFAULT_PATCH_CUT_PARAMS: &PatchCutParams = &PatchCutParams {
            atomic_cuts: PerAtomicPatchCut::new(),
        };

        self.by_patch
            .get(&index)
            .unwrap_or(DEFAULT_PATCH_CUT_PARAMS)
    }
}
impl IndexMut<PatchId> for CutParams {
    fn index_mut(&mut self, index: PatchId) -> &mut Self::Output {
        self.in_patch(index)
    }
}

#[derive(Debug, Default)]
pub struct PatchCutParams {
    pub(super) atomic_cuts: PerAtomicPatchCut<AtomicPatchCutParams>,
}
impl Index<AtomicPatchCutId> for PatchCutParams {
    type Output = AtomicPatchCutParams;

    fn index(&self, index: AtomicPatchCutId) -> &Self::Output {
        &self.atomic_cuts[index]
    }
}
impl IndexMut<AtomicPatchCutId> for PatchCutParams {
    fn index_mut(&mut self, index: AtomicPatchCutId) -> &mut Self::Output {
        &mut self.atomic_cuts[index]
    }
}

#[derive(Debug)]
pub(super) struct AtomicPatchCutParams {
    pub(super) cut: AtomicCutParams,
    pub(super) portal_interactions: PerPortal<Option<PortalCutInteraction>>,
}
impl AtomicPatchCutParams {
    pub fn new(cut: AtomicCutParams, portal_count: usize) -> Self {
        AtomicPatchCutParams {
            cut,
            portal_interactions: PerPortal::from(vec![None; portal_count]),
        }
    }
}
impl Index<PortalId> for AtomicPatchCutParams {
    type Output = Option<PortalCutInteraction>;

    fn index(&self, index: PortalId) -> &Self::Output {
        &self.portal_interactions[index]
    }
}
impl IndexMut<PortalId> for AtomicPatchCutParams {
    fn index_mut(&mut self, index: PortalId) -> &mut Self::Output {
        &mut self.portal_interactions[index]
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub(super) enum PortalCutInteraction {
    /// The portal is inside the cut and not touching.
    Inside,
    /// The portal is outside the cut and not touching.
    Outside,
    /// The portal touches the cut. This variant contains the extension of the
    /// cut on the other side of the portal.
    Extension(AtomicPatchCutId),
}

idx_struct! {
    /// ID of an atomic cut within a patch.
    pub(super) struct AtomicPatchCutId(u8);
    /// ID of a region within a patch bounded by atomic cuts.
    ///
    /// Each bit corresponds to an atomic cut within the patch:
    /// - LSb = first cut, MSb = last cut
    /// - 1 = **i**nside, 0 = **o**utside
    pub(super) struct AtomicPatchCutRegionId(u16);
    /// ID of a region comprised of multiple atomic patch cut regions.
    ///
    /// This is a bitset where each bit corresponds to a cut in the region. 1
    /// indicates that the region is inside the cut and 0 indicates that the
    /// region is outside the cut.
    pub struct GlobalCutRegionId(u16);
    /// ID of a facet of a particular global cut region.
    pub(super) struct CutRegionFacetId(u16);
}
/// List containing a value per atomic cut within a patch.
pub(super) type PerAtomicPatchCut<T> = GenericVec<AtomicPatchCutId, T>;
/// List containing a value per atomic cut region within a patch.
pub(super) type PerAtomicPatchCutRegion<T> = GenericVec<AtomicPatchCutRegionId, T>;
/// List containing a value per global cut region.
pub type PerGlobalCutRegion<T> = GenericVec<GlobalCutRegionId, T>;

impl AtomicPatchCutRegionId {
    pub(super) fn is_inside(self, cut: AtomicPatchCutId) -> bool {
        self.0 & (1 << cut.0) != 0
    }
    pub(super) fn is_outside(self, cut: AtomicPatchCutId) -> bool {
        self.0 & (1 << cut.0) == 0
    }
    pub(super) fn set_inside(&mut self, cut: AtomicPatchCutId) {
        self.0 |= 1 << cut.0;
    }
    pub(super) fn set_outside(&mut self, cut: AtomicPatchCutId) {
        self.0 &= !(1 << cut.0);
    }
    pub(super) fn and_inside(mut self, cut: AtomicPatchCutId) -> Self {
        self.set_inside(cut);
        self
    }
    pub(super) fn and_outside(mut self, cut: AtomicPatchCutId) -> Self {
        self.set_outside(cut);
        self
    }
}

fn send_atomic_patch_cut_region_thru_portal(
    patches: &PatchGraph,
    cut_params: &CutParams,
    patch_id: PatchId,
    portal_id: PortalId,
    start_region: AtomicPatchCutRegionId,
) -> Result<Option<(PatchId, AtomicPatchCutRegionId)>> {
    let mut destination_region = AtomicPatchCutRegionId(0);
    let portal = &patches[patch_id].portals[portal_id];
    let destination_patch = portal.other_patch;

    // How does each cut interact with this portal?
    for (atomic_patch_cut_id, atomic_patch_cut) in &cut_params[patch_id].atomic_cuts {
        match atomic_patch_cut.portal_interactions[portal_id]
            .ok_or_else(|| eyre!("unknown portal interaction"))?
        {
            // If the whole portal is on the correct side of
            // this cut, then this cut doesn't affect the
            // destination region.
            PortalCutInteraction::Inside if start_region.is_inside(atomic_patch_cut_id) => {}
            PortalCutInteraction::Outside if start_region.is_outside(atomic_patch_cut_id) => {}

            // If it's on the wrong side, then the region
            // doesn't cross the portal at all.
            PortalCutInteraction::Inside | PortalCutInteraction::Outside => return Ok(None),

            // If the cut exends through the portal, then we
            // have to cross through the portal and get on the
            // right side of the matching cut.
            PortalCutInteraction::Extension(matching_cut) => {
                if start_region.is_inside(atomic_patch_cut_id) {
                    destination_region.set_inside(matching_cut);
                } else {
                    destination_region.set_outside(matching_cut);
                }
            }
        };
    }

    // In the other patch, there may be some cuts that don't
    // intersect the portal. Make sure we're on the correct side
    // of those cuts.
    for (atomic_patch_cut_id, atomic_patch_cut) in &cut_params[destination_patch].atomic_cuts {
        match atomic_patch_cut.portal_interactions[portal_id]
            .ok_or_else(|| eyre!("unknown portal interaction"))?
        {
            PortalCutInteraction::Inside => {
                destination_region.set_inside(atomic_patch_cut_id);
            }
            PortalCutInteraction::Outside => {
                destination_region.set_outside(atomic_patch_cut_id);
            }
            PortalCutInteraction::Extension(_) => (), // already handled
        }
    }

    Ok(Some((destination_patch, destination_region)))
}
