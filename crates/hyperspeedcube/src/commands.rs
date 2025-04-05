//! Commands to grip and manipulate parts of the puzzle.

use std::fmt;
use std::str::FromStr;

use hyperpuzzle::prelude::*;
use itertools::Itertools;
use serde::{Deserialize, Serialize, de};

#[derive(Serialize, Deserialize, Debug, Default, Clone, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Command {
    // File menu (local)
    Open,
    Save,
    SaveAs,
    Exit,

    // File menu (web)
    CopyHscLog,
    CopyMc4dLog,
    PasteLog,

    // Edit menu
    Undo,
    Redo,
    Reset,

    // Scramble menu
    ScrambleN(usize),
    ScrambleFull,

    // Puzzle menu
    NewPuzzle(String),

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

            Command::CopyHscLog => "🗐".to_owned(),
            Command::CopyMc4dLog => "🗐".to_owned(),
            Command::PasteLog => "📋".to_owned(),

            Command::Undo => "⮪".to_owned(),
            Command::Redo => "⮫".to_owned(),
            Command::Reset => "⟲".to_owned(),

            Command::ScrambleN(n) => format!("🔀 {n}"),
            Command::ScrambleFull => "🔀".to_owned(),

            Command::NewPuzzle(ty) => format!("New {ty}"), // TODO: convert ID to name

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
    ViewPreset {
        #[serde(default)]
        view_preset_name: String,
    },

    #[default]
    #[serde(other)]
    None,
}
impl PuzzleCommand {
    pub fn short_description(&self, ty: &Puzzle) -> String {
        match self {
            PuzzleCommand::Grip { axis, layers } => {
                // IIFE to mimic try_block
                (|| {
                    let axis_id = ty.axis_names.id_from_name(axis.as_ref()?)?;
                    let axis_info = &ty.axes[axis_id];
                    let layer_mask = layers.to_layer_mask(axis_info);
                    let axis_name = &ty.axis_names[axis_id];
                    if layer_mask == LayerMask(0) {
                        Some(axis_name.to_owned())
                    } else {
                        Some(layer_mask.to_string() + axis_name)
                    }
                })()
                .unwrap_or_else(|| layers.to_string())
            }
            PuzzleCommand::Twist {
                axis: _,
                direction: _,
                layers: _,
            } => todo!("description of twist command"),
            PuzzleCommand::Recenter { axis } => {
                // IIFE to mimic try_block
                match (|| ty.axis_names.id_from_name(axis.as_ref()?))() {
                    Some(_twist_axis) => todo!("description of recenter command"),
                    None => "Recenter".to_string(),
                }
            }

            PuzzleCommand::Filter { mode, filter_name } => match filter_name.as_str() {
                "Next" => "➡".to_string(),
                "Previous" => "⬅".to_string(),
                _ => match mode {
                    FilterMode::ShowExactly => "👁".to_string(),
                    FilterMode::Show => "👁".to_string(),
                    FilterMode::Hide => "ｘ".to_string(),
                    FilterMode::HideAllExcept => "❎".to_string(),
                    FilterMode::Toggle => "~".to_string(),
                },
            },

            PuzzleCommand::KeybindSet { keybind_set_name } => keybind_set_name.to_string(),
            PuzzleCommand::ViewPreset { view_preset_name } => view_preset_name.to_string(),

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
    pub fn view_preset_name_mut(&mut self) -> Option<&mut String> {
        match self {
            Self::ViewPreset {
                view_preset_name, ..
            } => Some(view_preset_name),
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

    pub(crate) fn to_layer_mask(&self, axis: &AxisInfo) -> LayerMask {
        let mut ret = LayerMask(0);

        let layer_count = axis.layers.len() as u8;

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
