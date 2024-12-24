use itertools::Itertools;
use serde::{Deserialize, Serialize};
use strum::{Display, EnumIter, EnumMessage};

use super::{LayerMask, LayeredTwist, Puzzle};

/// Convention for counting moves.
#[allow(missing_docs)]
#[derive(
    Serialize,
    Deserialize,
    Debug,
    Default,
    Display,
    EnumIter,
    EnumMessage,
    Copy,
    Clone,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
)]
#[serde(rename_all = "UPPERCASE")]
pub enum TwistMetric {
    #[strum(serialize = "ATM", message = "Axial Turn Metric")]
    Atm,
    #[strum(serialize = "ETM", message = "Execution Turn Metric")]
    Etm,

    #[default]
    #[strum(serialize = "STM", message = "Slice Turn Metric (default)")]
    Stm,
    #[strum(serialize = "BTM", message = "Block Turn Metric")]
    Btm,
    #[strum(serialize = "OBTM", message = "Outer Block Turn Metric")]
    Obtm,

    #[strum(serialize = "QSTM", message = "Quarter Slice Turn Metric")]
    Qstm,
    #[strum(serialize = "QBTM", message = "Quarter Block Turn Metric")]
    Qbtm,
    #[strum(serialize = "QOBTM", message = "Quarter Outer Block Turn Metric")]
    Qobtm,
}
impl TwistMetric {
    /// Returns a long multiline description of how the turn metric works.
    pub fn long_description(self) -> String {
        let mut bullets = vec![];

        if self == Self::Atm {
            bullets.push(
                "Consecutive twists of the same axis are combined, even with different layers.",
            );
        }
        if self == Self::Etm {
            bullets
                .push("Twists are counted as they are executed, including whole-puzzle rotations.");
        } else {
            bullets.push("Whole-puzzle rotations are not counted.");
        }
        match self {
            Self::Stm | Self::Qstm => bullets.push("Slice twists count as one move."),
            Self::Btm | Self::Qbtm => {
                bullets.push("Noncontiguous slice twists are split into contiguous slice twists.");
            }
            Self::Obtm | Self::Qobtm => {
                bullets.push("Slice twists are split into contiguous outer-block twists.");
            }
            _ => (),
        }
        match self.is_qtm() {
            Some(false) => {
                bullets.push("Consecutive twists of the same axis and layers are combined.");
            }
            Some(true) => bullets.push("Double twists are split into quarters."),
            None => (),
        }

        bullets.into_iter().map(|s| format!("• {s}")).join("\n")
    }

    /// Returns `Some(true)` if the turn metric splits double twists into
    /// quarters, `Some(false)` if it combines consecutive twists of with the
    /// same axis and layers, or `None` if neither makes sense.
    pub fn is_qtm(self) -> Option<bool> {
        match self {
            Self::Atm | Self::Etm => None,
            Self::Stm | Self::Btm | Self::Obtm => Some(false),
            Self::Qstm | Self::Qbtm | Self::Qobtm => Some(true),
        }
    }
    /// Changes the turn metric to one that is the same except for how it
    /// handles quarter turns. This method has no effect on ATM and ETM.
    pub fn set_qtm(&mut self, is_qtm: bool) {
        *self = match self {
            Self::Stm | Self::Qstm => {
                if is_qtm {
                    Self::Qstm
                } else {
                    Self::Stm
                }
            }
            Self::Btm | Self::Qbtm => {
                if is_qtm {
                    Self::Qbtm
                } else {
                    Self::Btm
                }
            }
            Self::Obtm | Self::Qobtm => {
                if is_qtm {
                    Self::Qobtm
                } else {
                    Self::Obtm
                }
            }
            _ => *self,
        };
    }

    /// Counts a sequence of twists using this metric.
    pub fn count_twists(
        self,
        puzzle: &Puzzle,
        twists: impl IntoIterator<Item = LayeredTwist>,
    ) -> u64 {
        #[allow(clippy::needless_late_init)]
        let slice_multiplier: fn(LayerMask, u8) -> u32;

        match self {
            Self::Atm => {
                let mut count = 0;

                let mut prev_axis = None;
                for twist in twists {
                    let axis = puzzle.twists[twist.transform].axis;
                    let axis_info = &puzzle.axes[axis];
                    let opp = axis_info.opposite;
                    let is_same_axis = prev_axis == Some(axis) || opp.is_some() && prev_axis == opp;
                    if !is_same_axis {
                        count += 1;
                        prev_axis = Some(axis);
                    }
                }

                return count;
            }
            Self::Etm => return twists.into_iter().count() as u64,

            Self::Stm | Self::Qstm => {
                slice_multiplier = |_, _| 1;
            }
            Self::Btm | Self::Qbtm => {
                slice_multiplier = |layers, _| layers.count_contiguous_slices();
            }
            Self::Obtm | Self::Qobtm => {
                slice_multiplier = LayerMask::count_outer_blocks;
            }
        }

        let is_qtm = self.is_qtm().expect("ATM and ETM cases already handled");

        let mut count = 0;

        let mut prev_axis = None;
        let mut prev_layers = None;
        for twist in twists {
            let twist_info = &puzzle.twists[twist.transform];
            let axis = twist_info.axis;

            let direction_multiplier = if is_qtm {
                twist_info.qtm as u64
            } else if prev_axis == Some(axis) && prev_layers == Some(twist.layers) {
                // Same axis and layers as previous twist! This twist is
                // free.
                0
            } else {
                1
            };

            prev_axis = Some(axis);
            prev_layers = Some(twist.layers);

            let layer_count = puzzle.axes[axis].layers.len() as u8;
            count += direction_multiplier * slice_multiplier(twist.layers, layer_count) as u64;
        }

        count
    }
}
