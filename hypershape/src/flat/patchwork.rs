use super::*;

/// Patch of space containing polytopes.
pub type Patch<'a> = SpaceRef<'a, PatchId>;
/// Seam which bounds a patch of space and links to another seam, which may in
/// the same patch or a different patch, and may even be the same seam.
pub type Seam<'a> = SpaceRef<'a, SeamId>;
