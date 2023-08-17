use enum_iterator::Sequence;
use hypermath::prelude::*;
use hypershape::prelude::*;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::hash::Hash;
use strum::{Display, EnumIter, EnumMessage};

use super::*;

/// Convention for counting moves.
#[derive(
    Serialize,
    Deserialize,
    Debug,
    Default,
    Display,
    EnumIter,
    EnumMessage,
    Sequence,
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
    /// Axial Turn Metric
    #[strum(serialize = "ATM", message = "Axial Turn Metric")]
    Atm,
    /// Execution Turn Metric
    #[strum(serialize = "ETM", message = "Execution Turn Metric")]
    Etm,

    /// Slice Turn Metric (default)
    #[default]
    #[strum(serialize = "STM", message = "Slice Turn Metric (default)")]
    Stm,
    /// Block Turn Metric
    #[strum(serialize = "BTM", message = "Block Turn Metric")]
    Btm,
    /// Outer Block Turn Metric
    #[strum(serialize = "OBTM", message = "Outer Block Turn Metric")]
    Obtm,

    /// Quarter Slice Turn Metric
    #[strum(serialize = "QSTM", message = "Quarter Slice Turn Metric")]
    Qstm,
    /// Quarter Block Turn Metric
    #[strum(serialize = "QBTM", message = "Quarter Block Turn Metric")]
    Qbtm,
    /// Quarter Outer Block Turn Metric
    #[strum(serialize = "QOBTM", message = "Quarter Outer Block Turn Metric")]
    Qobtm,
}
impl TwistMetric {
    /// Returns a multiline explanation of the turn metric.
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
                bullets.push("Noncontiguous slice twists are split into contiguous slice twists.")
            }
            Self::Obtm | Self::Qobtm => {
                bullets.push("Slice twists are split into contiguous outer-block twists.")
            }
            _ => (),
        }
        match self.is_qtm() {
            Some(false) => {
                bullets.push("Consecutive twists of the same axis and layers are combined.")
            }
            Some(true) => bullets.push("Double twists are split into quarters."),
            None => (),
        }

        bullets.into_iter().map(|s| format!("• {s}")).join("\n")
    }

    /// Returns whether the metric is based on quarter turns.
    pub fn is_qtm(self) -> Option<bool> {
        match self {
            Self::Atm | Self::Etm => None,
            Self::Stm | Self::Btm | Self::Obtm => Some(false),
            Self::Qstm | Self::Qbtm | Self::Qobtm => Some(true),
        }
    }
    /// Returns whether the metric is based on quarter turns.
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

    /// Counts a sequence of twists using the metric.
    pub fn count_twists(
        self,
        puzzle: &PuzzleType,
        twists: impl IntoIterator<Item = Twist>,
    ) -> usize {
        #[allow(clippy::needless_late_init)]
        let slice_multiplier: fn(LayerMask, u8) -> u32;

        match self {
            Self::Atm => {
                let mut count = 0;

                let mut prev_axis = None;
                for twist in twists {
                    let axis = puzzle.info(twist.transform).axis;
                    let axis_info = &puzzle.info(axis);
                    let opposite_axis = axis_info.opposite;
                    let is_same_axis = prev_axis == Some(axis);
                    let is_opposite_axis = opposite_axis.is_some() && prev_axis == opposite_axis;
                    if !is_same_axis && !is_opposite_axis {
                        if twist.layers == axis_info.all_layers() {
                            // Axes may have shifted around, so clear them.
                            prev_axis = None;
                        } else {
                            count += 1;
                            prev_axis = Some(axis);
                        }
                    }
                }

                return count;
            }
            Self::Etm => return twists.into_iter().count(),

            Self::Stm | Self::Qstm => slice_multiplier = |_, _| 1,
            Self::Btm | Self::Qbtm => {
                slice_multiplier = |layers, _| layers.count_contiguous_slices()
            }
            Self::Obtm | Self::Qobtm => slice_multiplier = LayerMask::count_outer_blocks,
        }

        let is_qtm = self.is_qtm().unwrap();

        let mut count = 0;

        let mut prev_axis = None;
        let mut prev_layers = None;
        for twist in twists {
            let transform_info = &puzzle.info(twist.transform);
            let axis = transform_info.axis;
            let axis_info = puzzle.info(axis);
            if twist.layers == axis_info.all_layers() {
                let opposite_axis = axis_info.opposite;
                let is_same_axis = prev_axis == Some(axis);
                let is_opposite_axis = opposite_axis.is_some() && prev_axis == opposite_axis;
                if !is_same_axis && !is_opposite_axis {
                    // Axes may have shifted around, so clear them.
                    prev_axis = None;
                    prev_layers = None;
                }
                // Don't count full-puzzle rotations.
                continue;
            }

            prev_axis = Some(axis);
            prev_layers = Some(twist.layers);

            let direction_multiplier = if is_qtm {
                transform_info.qtm
            } else if prev_axis == Some(axis) && prev_layers == Some(twist.layers) {
                // Same axis and layers as previous twist! This twist is free.
                continue;
            } else {
                1
            };

            count += direction_multiplier
                * slice_multiplier(twist.layers, axis_info.layer_count()) as usize;
        }

        count
    }
}
