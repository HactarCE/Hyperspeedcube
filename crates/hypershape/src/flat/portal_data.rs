use super::*;

/// Description of a portal that is stored in a [`Space`].
///
/// Portals are used to implement quotients of Euclidean space. Portals always
/// represent reflections of space across the hyperplane.
pub struct PortalData {
    /// Portal boundary.
    ///
    /// The "outside" of the hyperplane is the side that may contain geometry.
    /// The "inside" does not exist.
    pub hyperplane: HyperplaneId,
}
