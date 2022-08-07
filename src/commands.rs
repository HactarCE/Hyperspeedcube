//! Commands to grip and manipulate parts of the puzzle.

use itertools::Itertools;
use serde::{de, Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

use crate::puzzle::*;

/// Minimum number of moves for a partial scramble.
pub const PARTIAL_SCRAMBLE_MOVE_COUNT_MIN: usize = 1;
/// Maximum number of moves for a partial scramble.
pub const PARTIAL_SCRAMBLE_MOVE_COUNT_MAX: usize = 20;

#[derive(Serialize, Deserialize, Debug, Default, Clone, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Command {
    // File menu
    Open,
    Save,
    SaveAs,
    Exit,

    // Edit menu
    Undo,
    Redo,
    Reset,

    // Scramble menu
    ScrambleN(usize),
    ScrambleFull,

    // Puzzle menu
    NewPuzzle(PuzzleTypeEnum),

    ToggleBlindfold,

    #[default]
    #[serde(other)]
    None,
}
impl Command {
    pub(crate) fn short_description(&self) -> String {
        match self {
            Command::Open => "🗁".to_owned(),
            Command::Save => "💾".to_owned(),
            Command::SaveAs => "Save As".to_owned(),
            Command::Exit => "Exit".to_owned(),

            Command::Undo => "⮪".to_owned(),
            Command::Redo => "⮫".to_owned(),
            Command::Reset => "⟲".to_owned(),

            Command::ScrambleN(n) => format!("🔀 {n}"),
            Command::ScrambleFull => "🔀".to_owned(),

            Command::NewPuzzle(ty) => format!("New {}", ty.name()),

            Command::ToggleBlindfold => "BLD".to_owned(),

            Command::None => String::new(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Default, Clone, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[allow(missing_docs)]
pub enum PuzzleCommand {
    GripAxis(String),
    GripLayers(LayerMaskDesc),
    Twist {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        axis: Option<String>,
        direction: String,
        layers: LayerMaskDesc,
    },
    Recenter {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        axis: Option<String>,
    },

    #[default]
    #[serde(other)]
    None,
}
impl PuzzleCommand {
    pub fn short_description(&self, ty: PuzzleTypeEnum) -> String {
        match self {
            PuzzleCommand::GripAxis(axis_name) => axis_name.to_owned(),
            PuzzleCommand::GripLayers(layers) => layers.to_string(),
            PuzzleCommand::Twist {
                axis,
                direction,
                layers,
            } => ty.twist_command_short_description(
                axis.as_deref()
                    .and_then(|axis_name| ty.twist_axis_from_name(axis_name)),
                ty.twist_direction_from_name(direction).unwrap_or_default(),
                layers.to_layer_mask(ty.layer_count()),
            ),

            PuzzleCommand::Recenter { axis } => {
                match axis
                    .as_deref()
                    .and_then(|axis_name| ty.twist_axis_from_name(axis_name))
                {
                    Some(twist_axis) => match ty.make_recenter_twist(twist_axis) {
                        Ok(twist) => ty.twist_command_short_description(
                            Some(twist.axis),
                            twist.direction,
                            twist.layers,
                        ),
                        Err(_) => crate::util::INVALID_STR.to_string(),
                    },
                    None => "Recenter".to_string(),
                }
            }

            PuzzleCommand::None => String::new(),
        }
    }
}

/// Description of a layer mask that adjusts to the size of a puzzle.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct LayerMaskDesc {
    segments: Vec<LayerMaskDescSegment>,
}
impl fmt::Display for LayerMaskDesc {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            self.segments.iter().map(|seg| seg.to_string()).join(","),
        )
    }
}
impl FromStr for LayerMaskDesc {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self {
            segments: s
                .split(',')
                .map(|segment_str| segment_str.parse())
                .collect::<Result<_, _>>()?,
        })
    }
}
impl Serialize for LayerMaskDesc {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.to_string().serialize(serializer)
    }
}
impl<'de> Deserialize<'de> for LayerMaskDesc {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        String::deserialize(deserializer)?
            .parse()
            .map_err(de::Error::custom)
    }
}
impl LayerMaskDesc {
    pub(crate) fn to_layer_mask(&self, layer_count: u8) -> LayerMask {
        let mut ret = LayerMask(0);

        fn layer_idx(i: i8, layer_count: u8) -> u8 {
            if i > 0 {
                i as u8 - 1
            } else {
                layer_count.saturating_sub(i.saturating_neg() as u8)
            }
        }

        for segment in &self.segments {
            let start = layer_idx(segment.start, layer_count);
            let end = layer_idx(segment.end, layer_count);
            let segment_mask = LayerMask::from(start..=end);
            if segment.subtract {
                ret = ret & !segment_mask;
            } else {
                ret = ret | segment_mask;
            }
        }

        ret & LayerMask::all_layers(layer_count)
    }
}

#[derive(Debug, Default, Copy, Clone, PartialEq, Eq)]
pub struct LayerMaskDescSegment {
    subtract: bool,
    start: i8,
    end: i8,
}
impl fmt::Display for LayerMaskDescSegment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.subtract {
            write!(f, "!")?;
        }
        write!(f, "{}", self.start)?;
        if self.start != self.end {
            write!(f, "..{}", self.end)?;
        }
        Ok(())
    }
}
impl FromStr for LayerMaskDescSegment {
    type Err = std::convert::Infallible;

    fn from_str(mut s: &str) -> Result<Self, Self::Err> {
        let subtract = match s.strip_prefix('!') {
            Some(rest) => {
                s = rest;
                true
            }
            None => false,
        };

        fn parse_i8(s: &str) -> i8 {
            use std::num::IntErrorKind::*;

            match s.trim().parse() {
                Ok(n) => n,
                Err(e) => match e.kind() {
                    PosOverflow => i8::MAX,
                    NegOverflow => i8::MIN,
                    _ => 0,
                },
            }
        }

        let (start, end) = match s.split_once("..") {
            Some((start_str, end_str)) => (parse_i8(start_str), parse_i8(end_str)),
            None => {
                let n = parse_i8(s);
                (n, n)
            }
        };

        Ok(Self {
            subtract,
            start,
            end,
        })
    }
}
