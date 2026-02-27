use hyperpuzzle::Timestamp;
use hyperpuzzle::prelude::*;
use smallvec::SmallVec;

/// Action on a puzzle that is tracked in the undo/redo history.
#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    /// Scramble twists. Cannot be undone.
    Scramble {
        /// Time that the scramble became visible to the user, which may be
        /// slightly different from the timestamp used to generate the scramble.
        time: Option<Timestamp>,
    },
    /// Sequence of twists executed as a single unit. This is usually a single
    /// twist.
    Twists {
        /// Move counter before the twist sequence.
        old_stm_counter: StmCounter,
        /// Twist sequence.
        twists: SmallVec<[LayeredTwist; 4]>,
    },
    /// Start of solve. Cannot be undone.
    StartSolve {
        /// Event timestamp.
        time: Option<Timestamp>,
        /// Log file duration at the time.
        duration: Option<i64>,
    },
    /// End of solve. Undone automatically when undoing twists.
    EndSolve {
        /// Event timestamp.
        time: Option<Timestamp>,
        /// Log file duration at the time.
        duration: Option<i64>,
    },
}
impl Action {
    /// Returns the undo behavior for the action.
    pub(crate) fn undo_behavior(&self) -> UndoBehavior {
        match self {
            Action::Scramble { .. } => UndoBehavior::Boundary,
            Action::Twists { .. } => UndoBehavior::Action,
            Action::StartSolve { .. } => UndoBehavior::Boundary,
            Action::EndSolve { .. } => UndoBehavior::Marker,
        }
    }
}

pub(crate) enum UndoBehavior {
    /// Action: can be undone and redone.
    Action,
    /// Marker: is automatically undone along with the preceding action, and is
    /// never part of the undo history.
    Marker,
    /// Boundary: cannot be undone.
    Boundary,
}
