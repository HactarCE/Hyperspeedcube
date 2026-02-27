use crate::{Axis, LayerMask};

use super::{LayeredTwist, Puzzle};

// TODO: move this to hypuz_notation

/// Counts a sequence of twists using Slice Turn Metric.
pub fn count_stm(puzzle: &Puzzle, twists: impl IntoIterator<Item = LayeredTwist>) -> u64 {
    let mut counter = StmCounter::new();
    for twist in twists {
        let twist_info = &puzzle.twists.twists[twist.transform];
        let axis = twist_info.axis;
        counter.count_twist(axis, twist.layers);
    }
    counter.count
}

/// Slice Turn Metric counter.
///
/// This type is usually cheap to clone.
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
pub struct StmCounter {
    /// Number of moves counted.
    pub count: u64,
    /// Axis and layer mask of the last move, or `None` if the last move is
    /// nonexistent/unknown.
    pub last_move: Option<(Axis, LayerMask)>,
}

impl StmCounter {
    /// Constructs a blank STM counter.
    pub fn new() -> Self {
        Self::default()
    }

    /// Constructs an STM counter with the given last move.
    pub fn with_last_twist(count: u64, last_move: Option<(Axis, LayerMask)>) -> Self {
        Self { count, last_move }
    }

    /// Counts a twist.
    pub fn count_twist(&mut self, axis: Axis, layer_mask: LayerMask) {
        if self.last_move != Some((axis, layer_mask)) {
            self.count += 1;
            self.last_move = Some((axis, layer_mask));
        }
    }

    /// Resets the count and the last move.
    pub fn reset(&mut self) {
        *self = Self::new();
    }
}
