use serde::{Deserialize, Serialize};
use strum::{AsRefStr, Display, EnumIter, EnumMessage};

#[derive(
    AsRefStr,
    Display,
    EnumIter,
    EnumMessage,
    Serialize,
    Deserialize,
    Debug,
    Copy,
    Clone,
    PartialEq,
    Eq,
    Hash,
)]
#[serde(rename_all = "UPPERCASE")]
pub enum TwistMetric {
    /// Quarter Slice Turn Metric: Each twist counts separately. Whole-puzzle
    /// rotations are not counted.
    #[strum(
        serialize = "QSTM",
        message = "Quarter Slice Turn Metric: Each twist counts separately. Whole-puzzle rotations are not counted."
    )]
    Qstm,

    /// Face Turn Metric: Consecutive twists with the same face and layer mask
    /// are combined.
    #[strum(
        serialize = "FTM",
        message = "Face Turn Metric: Consecutive twists with the same face and layer mask are combined."
    )]
    Ftm,

    /// Slice Turn Metric: Consecutive twists with the same face are combined,
    /// even with different layers.
    #[strum(
        serialize = "STM",
        message = "Slice Turn Metric: Consecutive twists with the same face are combined, even with different layers."
    )]
    Stm,

    /// Execution Turn Metric: Each twist is counted separately.
    #[strum(
        serialize = "ETM",
        message = "Execution Turn Metric: Each twist is counted separately."
    )]
    Etm,
}
impl Default for TwistMetric {
    fn default() -> Self {
        Self::Qstm
    }
}
