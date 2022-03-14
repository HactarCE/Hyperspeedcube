use serde::{Deserialize, Serialize};
use strum::{Display, EnumIter, EnumMessage};

/// Convention for counting moves.
#[derive(
    Serialize, Deserialize, Debug, Display, EnumIter, EnumMessage, Copy, Clone, PartialEq, Eq, Hash,
)]
#[serde(rename_all = "UPPERCASE")]
pub enum TwistMetric {
    /// Quarter Slice Turn Metric: Each twist counts separately. Whole-puzzle
    /// rotations are not counted.
    #[strum(
        serialize = "QSTM",
        message = "Quarter Slice Turn Metric",
        detailed_message = "Each twist counts separately. Whole-puzzle rotations are not counted."
    )]
    Qstm,

    /// Face Turn Metric: Consecutive twists with the same face and layers are
    /// combined.
    #[strum(
        serialize = "FTM",
        message = "Face Turn Metric",
        detailed_message = "Consecutive twists with the same face and layers are combined."
    )]
    Ftm,

    /// Slice Turn Metric: Consecutive twists with the same face are combined,
    /// even with different layers.
    #[strum(
        serialize = "STM",
        message = "Slice Turn Metric",
        detailed_message = "Consecutive twists with the same face are combined, even with different layers."
    )]
    Stm,

    /// Execution Turn Metric: Each twist counts separately, including
    /// whole-puzzle rotations.
    #[strum(
        serialize = "ETM",
        message = "Execution Turn Metric",
        detailed_message = "Each twist counts separately, including whole-puzzle rotations."
    )]
    Etm,
}
impl Default for TwistMetric {
    fn default() -> Self {
        Self::Qstm
    }
}
impl TwistMetric {
    /// Returns the next twist metric in a cycle.
    pub fn next(self) -> Self {
        match self {
            Self::Qstm => Self::Ftm,
            Self::Ftm => Self::Stm,
            Self::Stm => Self::Etm,
            Self::Etm => Self::Qstm,
        }
    }
}
