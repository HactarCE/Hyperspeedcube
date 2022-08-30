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

#[derive(Serialize, Deserialize, Debug, Default, Clone, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum PuzzleMouseCommand {
    TwistCw,
    TwistCcw,
    Recenter,
    SelectPiece,

    #[default]
    #[serde(other)]
    None,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PuzzleCommand {
    Grip {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        axis: Option<String>,
        #[serde(default, skip_serializing_if = "LayerMaskDesc::is_default")]
        layers: LayerMaskDesc,
    },
    Twist {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        axis: Option<String>,
        #[serde(default)]
        direction: String,
        #[serde(default)]
        layers: LayerMaskDesc,
    },
    Recenter {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        axis: Option<String>,
    },

    Filter {
        #[serde(default)]
        mode: FilterMode,
        #[serde(default)]
        filter_name: String,
    },

    KeybindSet {
        #[serde(default)]
        keybind_set_name: String,
    },

    #[default]
    #[serde(other)]
    None,
}
impl PuzzleCommand {
    pub fn short_description(&self, ty: PuzzleTypeEnum) -> String {
        match self {
            PuzzleCommand::Grip { axis, layers } => {
                let layers = layers.to_layer_mask(ty.layer_count());
                let mut s = String::new();
                if layers != LayerMask(0) || axis.is_none() {
                    s += &layers.to_string();
                }
                if let Some(axis_name) = axis {
                    s += axis_name;
                }
                s
            }
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

            PuzzleCommand::Filter { mode, filter_name } => format!("{mode} {filter_name}"),

            PuzzleCommand::KeybindSet { keybind_set_name } => format!("{keybind_set_name}"),

            PuzzleCommand::None => String::new(),
        }
    }

    pub fn layers_mut(&mut self) -> Option<&mut LayerMaskDesc> {
        match self {
            Self::Grip { layers, .. } | Self::Twist { layers, .. } => Some(layers),
            _ => None,
        }
    }
    pub fn axis_mut(&mut self) -> Option<&mut Option<String>> {
        match self {
            Self::Grip { axis, .. } | Self::Twist { axis, .. } | Self::Recenter { axis } => {
                Some(axis)
            }
            _ => None,
        }
    }
    pub fn direction_mut(&mut self) -> Option<&mut String> {
        match self {
            Self::Twist { direction, .. } => Some(direction),
            _ => None,
        }
    }
    pub fn filter_mode_mut(&mut self) -> Option<&mut FilterMode> {
        match self {
            Self::Filter { mode, .. } => Some(mode),
            _ => None,
        }
    }
    pub fn filter_name_mut(&mut self) -> Option<&mut String> {
        match self {
            Self::Filter { filter_name, .. } => Some(filter_name),
            _ => None,
        }
    }
    pub fn keybind_set_name_mut(&mut self) -> Option<&mut String> {
        match self {
            Self::KeybindSet {
                keybind_set_name, ..
            } => Some(keybind_set_name),
            _ => None,
        }
    }
}

/// Mode in which to apply a piece filter.
///
/// TODO: remove aliases (support for v0.9.0 preferences)
#[derive(
    Serialize,
    Deserialize,
    Debug,
    Display,
    AsRefStr,
    IntoStaticStr,
    EnumIter,
    Default,
    Copy,
    Clone,
    PartialEq,
    Eq,
    Hash,
)]
#[serde(rename_all = "snake_case")]
pub enum FilterMode {
    #[default]
    #[strum(serialize = "Show exactly")]
    #[serde(alias = "ShowExactly")]
    ShowExactly,
    #[strum(serialize = "Show")]
    #[serde(alias = "Show")]
    Show,
    #[strum(serialize = "Hide")]
    #[serde(alias = "Hide")]
    Hide,
    #[strum(serialize = "Hide all except")]
    #[serde(alias = "HideAllExcept")]
    HideAllExcept,
    #[strum(serialize = "Toggle")]
    #[serde(alias = "Toggle")]
    Toggle,
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
                .filter(|&segment| segment != Ok(LayerMaskDescSegment::default()))
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
    pub(crate) fn is_default(&self) -> bool {
        *self == Self::default()
    }

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
                ret &= !segment_mask;
            } else {
                ret |= segment_mask;
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
