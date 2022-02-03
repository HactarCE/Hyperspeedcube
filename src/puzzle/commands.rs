//! Commands to select and manipulate parts of the puzzle.

use serde::{Deserialize, Serialize};
use std::borrow::Cow;

use super::{traits::*, Face, LayerMask, PieceType, PuzzleType, TwistDirection};
use crate::preferences::DeserializePerPuzzle;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum SelectThing {
    Face(Face),
    Layers(LayerMask),
    PieceType(PieceType),
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
            SelectThing::Layers(layers) => Self::Layers(layers.0),
            SelectThing::PieceType(piece_type) => Self::PieceType(piece_type.name().into()),
        }
    }
}
impl<'de> DeserializePerPuzzle<'de> for SelectThing {
    type Proxy = SelectThingSerde<'de>;

    fn deserialize_from(thing: SelectThingSerde<'de>, ty: PuzzleType) -> Self {
        let layer_mask = (1 << ty.layer_count()) - 1;
        match thing {
            SelectThingSerde::Face(face) => Self::Face(Face::from_name(ty, &face)),
            SelectThingSerde::Layers(layers) => Self::Layers(LayerMask(layers & layer_mask)),
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

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum SelectHow {
    Hold,
    Toggle,
    Clear,
}

#[derive(Debug, Clone)]
#[allow(missing_docs)]
pub enum Command {
    Twist {
        face: Option<Face>,
        layers: LayerMask,
        direction: TwistDirection,
    },
    Recenter {
        face: Option<Face>,
    },

    HoldSelect(SelectThing),
    ToggleSelect(SelectThing),
    ClearToggleSelect(SelectCategory),

    None,
}
impl Serialize for Command {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        CommandSerde::from(self).serialize(serializer)
    }
}
impl Default for Command {
    fn default() -> Self {
        Self::None
    }
}
impl Command {
    pub(crate) fn get_select_category(&self) -> SelectCategory {
        match self {
            Command::HoldSelect(thing) | Command::ToggleSelect(thing) => thing.category(),
            Command::ClearToggleSelect(category) => *category,
            _ => SelectCategory::default(),
        }
    }
    pub(crate) fn get_select_thing(&self, ty: PuzzleType) -> SelectThing {
        match self {
            Command::HoldSelect(thing) | Command::ToggleSelect(thing) => *thing,
            Command::ClearToggleSelect(category) => SelectThing::default(*category, ty),
            _ => SelectThing::Face(ty.faces()[0]),
        }
    }
    pub(crate) fn get_select_how(&self) -> Option<SelectHow> {
        match self {
            Command::HoldSelect(_) => Some(SelectHow::Hold),
            Command::ToggleSelect(_) => Some(SelectHow::Toggle),
            Command::ClearToggleSelect(_) => Some(SelectHow::Clear),
            _ => None,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub enum CommandSerde<'a> {
    Twist {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        face: Option<Cow<'a, str>>,
        layers: u32,
        direction: Cow<'a, str>,
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
impl Default for CommandSerde<'_> {
    fn default() -> Self {
        Self::None
    }
}
impl<'a> From<&'a Command> for CommandSerde<'_> {
    fn from(command: &'a Command) -> Self {
        match command {
            Command::Twist {
                face,
                layers,
                direction,
            } => Self::Twist {
                face: face.map(|f| f.name().into()),
                layers: layers.0,
                direction: direction.name().into(),
            },
            Command::Recenter { face } => Self::Recenter {
                face: face.map(|f| f.name().into()),
            },

            Command::HoldSelect(thing) => Self::HoldSelect((*thing).into()),
            Command::ToggleSelect(thing) => Self::ToggleSelect((*thing).into()),
            Command::ClearToggleSelect(category) => Self::ClearToggleSelect(*category),

            Command::None => Self::None,
        }
    }
}
impl<'de> DeserializePerPuzzle<'de> for Command {
    type Proxy = CommandSerde<'de>;

    /// Checks that the command is valid, and modifies it to make it valid if it
    /// is not.
    fn deserialize_from(command: CommandSerde<'de>, ty: PuzzleType) -> Command {
        let layer_mask = (1 << ty.layer_count()) - 1;

        match command {
            CommandSerde::Twist {
                face,
                layers,
                direction,
            } => Self::Twist {
                face: face.map(|f| Face::from_name(ty, &f)),
                layers: LayerMask(layers & layer_mask),
                direction: TwistDirection::from_name(ty, &direction),
            },
            CommandSerde::Recenter { face } => Self::Recenter {
                face: face.map(|f| Face::from_name(ty, &f)),
            },

            CommandSerde::HoldSelect(thing) => {
                Self::HoldSelect(SelectThing::deserialize_from(thing, ty))
            }
            CommandSerde::ToggleSelect(thing) => {
                Self::ToggleSelect(SelectThing::deserialize_from(thing, ty))
            }
            CommandSerde::ClearToggleSelect(category) => Self::ClearToggleSelect(category),

            CommandSerde::None => Self::None,
        }
    }
}
