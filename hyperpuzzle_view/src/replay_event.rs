use hyperpuzzle::{LayerMask, LayeredTwist, Timestamp, Twist};
use smallvec::SmallVec;

/// Event that is part of a replay.
#[derive(Debug, Clone, PartialEq)]
pub enum ReplayEvent {
    /// Undo of the most recent undoable [`crate::Action`]
    Undo,
    /// Redo of the most recent redoable [`crate::Action`]
    Redo,
    /// Reset + scramble the puzzle
    Scramble,
    /// Click on a twist gizmo (does *not* actually apply the twist to the
    /// puzzle state)
    GizmoClick {
        /// Layers affected by the twist.
        layers: LayerMask,
        /// Gizmo target clicked on, which corresponds to a twist.
        target: Twist,
        /// Whether the twist should be executed in reverse.
        reverse: bool,
    },
    /// Click and drag to execute a twist (does *not* actually apply the twist
    /// to the puzzle state)
    DragTwist,
    /// Twist applied to the puzzle state
    Twists(SmallVec<[LayeredTwist; 4]>),
    /// Start of a solve (first move after being scrambled)
    StartSolve {
        /// Event timestamp
        time: Option<Timestamp>,
        /// Log file duration at the time
        duration: Option<i64>,
    },
    /// End of a solve (fully solved after being scrambled)
    EndSolve {
        /// Event timestamp
        time: Option<Timestamp>,
        /// Log file duration at the time
        duration: Option<i64>,
    },
    /// Start of a session
    StartSession {
        /// Event timestamp
        time: Option<Timestamp>,
    },
    /// End of a session
    EndSession {
        /// Event timestamp
        time: Option<Timestamp>,
    },
}
