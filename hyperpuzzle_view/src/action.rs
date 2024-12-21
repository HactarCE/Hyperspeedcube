use hyperpuzzle::{LayeredTwist, Timestamp};
use smallvec::SmallVec;

/// Action on a puzzle that is tracked in the undo/redo history.
#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    /// Scramble twists. Cannot be undone.
    Scramble,
    /// Sequence of twists executed as a single unit. This is usually a single
    /// twist.
    Twists(SmallVec<[LayeredTwist; 4]>),
    /// End of solve. Undone automatically when undoing twists.
    EndSolve {
        /// Event timestamp
        time: Timestamp,
    },
}
impl Action {
    /// Returns `true` for "marker" actions, which never appear in the redo
    /// stack and are automatically undone as part of the action before them.
    pub fn is_marker(&self) -> bool {
        match self {
            Action::Scramble => false,
            Action::Twists(_) => false,
            Action::EndSolve { .. } => true,
        }
    }
}
