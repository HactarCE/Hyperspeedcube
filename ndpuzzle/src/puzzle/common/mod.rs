#[macro_use]
mod info;
mod layers;
mod notation;
mod puzzle_state;
mod puzzle_type;
mod shape;
mod twist_metric;
mod twists;

pub use info::*;
pub use layers::LayerMask;
pub use notation::NotationScheme;
pub use puzzle_state::PuzzleState;
pub use puzzle_type::PuzzleType;
pub use shape::PuzzleShape;
pub use twist_metric::TwistMetric;
pub use twists::*;

use crate::math::Matrix;
use crate::LayerMaskUint;

/// Twists for the hovered sticker.
///
/// TODO: maybe remove
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, Hash)]
pub struct ClickTwists {
    /// Clockwise twist, typically bound to left click.
    pub cw: Option<Twist>,
    /// Counterclockwise twist, typically bound to right click.
    pub ccw: Option<Twist>,
    /// Recenter twist, typically bound to middle click.
    pub recenter: Option<Twist>,
}
impl ClickTwists {
    /// Swaps clockwise and counterclockwise.
    #[must_use]
    pub fn rev(self) -> Self {
        Self {
            cw: self.ccw,
            ccw: self.cw,
            ..self
        }
    }
}
