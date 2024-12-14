use hypermath::IndexOutOfRange;
use serde::{Deserialize, Serialize};

use super::{LayerMask, Puzzle, Twist};

/// Twist with layer mask.
#[derive(Serialize, Deserialize, Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct LayeredTwist {
    /// Layers.
    pub layers: LayerMask,
    /// Twist transform.
    pub transform: Twist,
}
impl LayeredTwist {
    /// Returns the reverse twist.
    pub fn rev(self, puzzle: &Puzzle) -> Result<Self, IndexOutOfRange> {
        let rev_transform = puzzle.twists.get(self.transform)?.reverse;
        Ok(LayeredTwist {
            layers: self.layers,
            transform: rev_transform,
        })
    }
}
