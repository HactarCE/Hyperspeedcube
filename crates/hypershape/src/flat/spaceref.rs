use super::*;

/// Reference to an element or other object in a space.
#[derive(Debug, Copy, Clone)]
pub struct SpaceRef<'a, I> {
    /// Space containing the object.
    pub(super) space: &'a Space,
    /// ID of the object.
    pub(super) id: I,
}
impl<I: fmt::Display> fmt::Display for SpaceRef<'_, I> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.id)
    }
}
impl<I: PartialEq> PartialEq for SpaceRef<'_, I> {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self.space, other.space) && self.id == other.id
    }
}
impl<I: Eq> Eq for SpaceRef<'_, I> {}
impl<I: Ord> Ord for SpaceRef<'_, I> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.id.cmp(&other.id)
    }
}
impl<I: PartialOrd> PartialOrd for SpaceRef<'_, I> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.id.partial_cmp(&other.id)
    }
}
impl<'a, I> SpaceRef<'a, I> {
    pub(super) fn new(space: &'a Space, id: I) -> Self {
        Self { space, id }
    }

    /// Returns the space containing the object.
    pub fn space(self) -> &'a Space {
        self.space
    }
    /// Returns the ID of the object.
    pub fn id(self) -> I
    where
        I: Clone,
    {
        self.id.clone()
    }

    /// Applies a function to the ID and returns a reference to a new object.
    pub fn map_id<J>(self, f: impl FnOnce(I) -> J) -> SpaceRef<'a, J> {
        SpaceRef {
            space: self.space,
            id: f(self.id),
        }
    }
}
