use serde::{Deserialize, Serialize};

use super::{LayerMask, Twist};

/// Twist with layer mask.
#[derive(Serialize, Deserialize, Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct LayeredTwist {
    /// Layers.
    pub layers: LayerMask,
    /// Twist transform.
    pub transform: Twist,
}
