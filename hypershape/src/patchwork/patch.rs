use eyre::Result;
use std::ops::{Index, IndexMut};

use hypermath::collections::generic_vec::IndexOutOfRange;
use hypermath::prelude::*;

use super::*;

#[derive(Debug, Clone)]
pub struct PatchData {
    pub portals: PerPortal<Portal>,
    pub schlafli: SchlafliSymbol,
    pub polytope: AtomicPolytopeRef,

    /// Root patch connecting to this patch.
    pub root: PatchId,
}
impl Index<PortalId> for PatchData {
    type Output = Portal;

    fn index(&self, index: PortalId) -> &Self::Output {
        &self.portals[index]
    }
}
impl IndexMut<PortalId> for PatchData {
    fn index_mut(&mut self, index: PortalId) -> &mut Self::Output {
        &mut self.portals[index]
    }
}

#[derive(Debug, Clone)]
pub struct Portal {
    /// Isometry
    pub isometry: Isometry,
    pub other_patch: PatchId,
    pub portal_of_other_patch: PortalId,
    pub polytope: AtomicPolytopeRef,
}

#[derive(Debug, Clone)]
pub struct PatchGraph {
    patches: PerPatch<PatchData>,
}

impl Index<PatchId> for PatchGraph {
    type Output = PatchData;

    fn index(&self, index: PatchId) -> &Self::Output {
        &self.patches[index]
    }
}
impl IndexMut<PatchId> for PatchGraph {
    fn index_mut(&mut self, index: PatchId) -> &mut Self::Output {
        &mut self.patches[index]
    }
}

impl PatchGraph {
    pub fn new(polytope: AtomicPolytopeRef) -> Result<Self> {
        let mut patches = PerPatch::new();
        let _default_patch = patches.push(PatchData {
            portals: PerPortal::new(),
            schlafli: SchlafliSymbol::from_indices(vec![]),
            polytope,

            root: patches.next_idx()?,
        })?;
        Ok(PatchGraph { patches })
    }

    pub fn add_schlafli_patch(
        &mut self,
        polytope: AtomicPolytopeRef,
        schlafli: SchlafliSymbol,
    ) -> Result<PatchId, IndexOutOfRange> {
        self.patches.push(PatchData {
            portals: PerPortal::new(),
            schlafli,
            polytope,

            root: self.patches.next_idx()?,
        })
    }
}
