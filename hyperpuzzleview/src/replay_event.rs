use hyperpuzzle::{LayerMask, LayeredTwist, Timestamp, Twist};
use hyperpuzzlelog::Scramble;
use smallvec::SmallVec;

/// Event that is part of a replay.
#[derive(Debug, Clone, PartialEq)]
pub enum ReplayEvent {
    /// Undo of the most recent undoable [`crate::Action`]
    Undo,
    /// Redo of the most recent redoable [`crate::Action`]
    Redo,
    /// Reset + scramble the puzzle
    Scramble(Scramble),
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
    /// End of a solve (fully solved after being scrambled)
    EndSolve {
        /// Event timestamp
        time: Timestamp,
    },
    /// End of a session (save + reload from disk)
    EndSession {
        /// Event timestamp
        time: Timestamp,
    },
}
