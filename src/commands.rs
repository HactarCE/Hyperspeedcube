//! Commands to select and manipulate parts of the puzzle.

use serde::{Deserialize, Serialize};
use std::borrow::Cow;

use crate::preferences::DeserializePerPuzzle;
use crate::puzzle::{traits::*, Face, LayerMask, PieceType, PuzzleType, Selection, TwistDirection};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
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

    // Puzzle menu
    NewPuzzle(PuzzleType),

    #[serde(other)]
    None,
}
impl Default for Command {
    fn default() -> Self {
        Self::None
    }
}
impl Command {
    pub(crate) fn get_puzzle_type(&self) -> PuzzleType {
        match self {
            Command::NewPuzzle(puzzle_type) => *puzzle_type,
            _ => PuzzleType::default(),
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum SelectThing {
    Face(Face),
    Layers(LayerMask),
    PieceType(PieceType),
}
impl From<SelectThing> for Selection {
    fn from(thing: SelectThing) -> Self {
        let mut ret = Selection {
            face_mask: 0,
            layer_mask: 0,
            piece_type_mask: 0,
        };
        match thing {
            SelectThing::Face(face) => ret.face_mask = 1 << face.id(),
            SelectThing::Layers(layer_mask) => ret.layer_mask = layer_mask.0,
            SelectThing::PieceType(piece_type) => ret.piece_type_mask = 1 << piece_type.id(),
        }
        ret
    }
}
impl SelectThing {
    fn category(self) -> SelectCategory {
        match self {
            Self::Face(_) => SelectCategory::Face,
            Self::Layers(_) => SelectCategory::Layers,
            Self::PieceType(_) => SelectCategory::PieceType,
        }
    }
    pub(crate) fn default(category: SelectCategory, ty: PuzzleType) -> Self {
        match category {
            SelectCategory::Face => Self::Face(ty.faces()[0]),
            SelectCategory::Layers => Self::Layers(LayerMask(1)),
            SelectCategory::PieceType => Self::PieceType(PieceType::default(ty)),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub enum SelectThingSerde<'a> {
    Face(Cow<'a, str>),
    Layers(u32),
    PieceType(Cow<'a, str>),
}
impl From<SelectThing> for SelectThingSerde<'_> {
    fn from(thing: SelectThing) -> Self {
        match thing {
            SelectThing::Face(face) => Self::Face(face.name().into()),
            SelectThing::Layers(layer_mask) => Self::Layers(layer_mask.0),
            SelectThing::PieceType(piece_type) => Self::PieceType(piece_type.name().into()),
        }
    }
}
impl<'de> DeserializePerPuzzle<'de> for SelectThing {
    type Proxy = SelectThingSerde<'de>;

    fn deserialize_from(thing: SelectThingSerde<'de>, ty: PuzzleType) -> Self {
        let total_layer_mask = (1 << ty.layer_count()) - 1;
        match thing {
            SelectThingSerde::Face(face) => Self::Face(Face::from_name(ty, &face)),
            SelectThingSerde::Layers(layer_mask) => {
                Self::Layers(LayerMask(layer_mask & total_layer_mask))
            }
            SelectThingSerde::PieceType(piece_type) => {
                Self::PieceType(PieceType::from_name(ty, &piece_type))
            }
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Display, EnumIter, Copy, Clone, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SelectCategory {
    Face,
    Layers,
    #[strum(serialize = "Piece type")]
    PieceType,
}
impl Default for SelectCategory {
    fn default() -> Self {
        Self::Face
    }
}

#[derive(Debug, Display, Copy, Clone, PartialEq, Eq)]
pub enum SelectHow {
    Hold,
    Toggle,
    Clear,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(missing_docs)]
pub enum PuzzleCommand {
    Twist {
        face: Option<Face>,
        direction: TwistDirection,
        layer_mask: LayerMask,
    },
    Recenter {
        face: Option<Face>,
    },

    HoldSelect(SelectThing),
    ToggleSelect(SelectThing),
    ClearToggleSelect(SelectCategory),

    None,
}
impl Serialize for PuzzleCommand {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        PuzzleCommandSerde::from(self).serialize(serializer)
    }
}
impl Default for PuzzleCommand {
    fn default() -> Self {
        Self::None
    }
}
impl PuzzleCommand {
    pub(crate) fn get_select_category(&self) -> SelectCategory {
        match self {
            PuzzleCommand::HoldSelect(thing) | PuzzleCommand::ToggleSelect(thing) => {
                thing.category()
            }
            PuzzleCommand::ClearToggleSelect(category) => *category,
            _ => SelectCategory::default(),
        }
    }
    pub(crate) fn get_select_thing(&self, ty: PuzzleType) -> SelectThing {
        match self {
            PuzzleCommand::HoldSelect(thing) | PuzzleCommand::ToggleSelect(thing) => *thing,
            PuzzleCommand::ClearToggleSelect(category) => SelectThing::default(*category, ty),
            _ => SelectThing::Face(ty.faces()[0]),
        }
    }
    pub(crate) fn get_select_how(&self) -> Option<SelectHow> {
        match self {
            PuzzleCommand::HoldSelect(_) => Some(SelectHow::Hold),
            PuzzleCommand::ToggleSelect(_) => Some(SelectHow::Toggle),
            PuzzleCommand::ClearToggleSelect(_) => Some(SelectHow::Clear),
            _ => None,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub enum PuzzleCommandSerde<'a> {
    Twist {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        face: Option<Cow<'a, str>>,
        direction: Cow<'a, str>,
        #[serde(rename = "layers")]
        layer_mask: u32,
    },
    Recenter {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        face: Option<Cow<'a, str>>,
    },

    HoldSelect(SelectThingSerde<'a>),
    ToggleSelect(SelectThingSerde<'a>),
    ClearToggleSelect(SelectCategory),

    #[serde(other)]
    None,
}
impl Default for PuzzleCommandSerde<'_> {
    fn default() -> Self {
        Self::None
    }
}
impl<'a> From<&'a PuzzleCommand> for PuzzleCommandSerde<'_> {
    fn from(command: &'a PuzzleCommand) -> Self {
        match command {
            PuzzleCommand::Twist {
                face,
                direction,
                layer_mask,
            } => Self::Twist {
                face: face.map(|f| f.name().into()),
                direction: direction.name().into(),
                layer_mask: layer_mask.0,
            },
            PuzzleCommand::Recenter { face } => Self::Recenter {
                face: face.map(|f| f.name().into()),
            },

            PuzzleCommand::HoldSelect(thing) => Self::HoldSelect((*thing).into()),
            PuzzleCommand::ToggleSelect(thing) => Self::ToggleSelect((*thing).into()),
            PuzzleCommand::ClearToggleSelect(category) => Self::ClearToggleSelect(*category),

            PuzzleCommand::None => Self::None,
        }
    }
}
impl<'de> DeserializePerPuzzle<'de> for PuzzleCommand {
    type Proxy = PuzzleCommandSerde<'de>;

    /// Checks that the command is valid, and modifies it to make it valid if it
    /// is not.
    fn deserialize_from(command: PuzzleCommandSerde<'de>, ty: PuzzleType) -> PuzzleCommand {
        let max_layer_mask = (1 << ty.layer_count()) - 1;

        match command {
            PuzzleCommandSerde::Twist {
                face,
                direction,
                layer_mask,
            } => Self::Twist {
                face: face.map(|f| Face::from_name(ty, &f)),
                direction: TwistDirection::from_name(ty, &direction),
                layer_mask: LayerMask(layer_mask & max_layer_mask),
            },
            PuzzleCommandSerde::Recenter { face } => Self::Recenter {
                face: face.map(|f| Face::from_name(ty, &f)),
            },

            PuzzleCommandSerde::HoldSelect(thing) => {
                Self::HoldSelect(SelectThing::deserialize_from(thing, ty))
            }
            PuzzleCommandSerde::ToggleSelect(thing) => {
                Self::ToggleSelect(SelectThing::deserialize_from(thing, ty))
            }
            PuzzleCommandSerde::ClearToggleSelect(category) => Self::ClearToggleSelect(category),

            PuzzleCommandSerde::None => Self::None,
        }
    }
}
