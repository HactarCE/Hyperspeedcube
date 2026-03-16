//! Commands to grip and manipulate parts of the puzzle.

use std::fmt;
use std::str::FromStr;

use hyperpuzzle::{notation::LayerPrefix, prelude::*};
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
        #[serde(default, skip_serializing_if = "is_layer_prefix_default")]
        layers: LayerPrefix,
    },
    Twist {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        axis: Option<String>,
        #[serde(default)]
        direction: String,
        #[serde(default)]
        layers: LayerPrefix,
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
                    let axis_id = ty.axes().names.id_from_name(axis.as_ref()?)?;
                    let layer_mask = layers.to_layer_mask(ty.axis_layers_info[axis_id]);
                    let axis_name = &ty.axes().names[axis_id];
                    if layer_mask.is_empty() {
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
                match (|| ty.axes().names.id_from_name(axis.as_ref()?))() {
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

    pub fn layers_mut(&mut self) -> Option<&mut LayerPrefix> {
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

fn is_layer_prefix_default(layer_prefix: &LayerPrefix) -> bool {
    *layer_prefix == LayerPrefix::default()
}
