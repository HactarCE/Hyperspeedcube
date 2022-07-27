//! Commands to select and manipulate parts of the puzzle.

use serde::{Deserialize, Serialize};

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
    SelectAxis(String),
    SelectLayers(LayerMask),
    Twist {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        axis: Option<String>,
        direction: String,
        layers: LayerMask,
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
            PuzzleCommand::SelectAxis(axis_name) => axis_name.to_owned(),
            PuzzleCommand::SelectLayers(layers) => layers.digits(),
            PuzzleCommand::Twist {
                axis,
                direction,
                layers,
            } => ty.twist_command_short_description(
                axis.as_deref()
                    .and_then(|axis_name| ty.twist_axis_from_name(axis_name)),
                ty.twist_direction_from_name(direction).unwrap_or_default(),
                *layers & ty.all_layers(),
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
