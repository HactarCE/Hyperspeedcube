use hyperpuzzle::LayeredTwist;
use smallvec::SmallVec;

/// Action on a puzzle that can be undone/redone.
#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    /// Sequence of twists executed as a single unit. This is usually a single
    /// twist.
    Twists(SmallVec<[LayeredTwist; 4]>),
}
