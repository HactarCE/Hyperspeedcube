//! Commands to select and manipulate parts of the puzzle.

use serde::{Deserialize, Serialize};

use super::PuzzleType;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
#[allow(missing_docs)]
pub enum Command {
    Twist {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        face: Option<String>,
        layers: LayerMask,
        direction: String,
    },
    Recenter {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        face: Option<String>,
    },

    HoldSelectFace(String),
    HoldSelectLayers(LayerMask),
    HoldSelectPieceType(PieceTypeId),
    ToggleSelectFace(String),
    ToggleSelectLayers(LayerMask),
    ToggleSelectPieceType(PieceTypeId),
    ClearToggleSelectFaces,
    ClearToggleSelectLayers,
    ClearToggleSelectPieceType,

    #[serde(other)]
    None,
}
impl Default for Command {
    fn default() -> Self {
        Self::None
    }
}
impl Command {
    /// Checks that the command is valid, and modifies it to make it valid if it
    /// is not.
    pub fn validate(&mut self, puz_type: PuzzleType) {
        let mut f = None;
        let mut l = None;
        let mut d = None;
        let mut p = None;

        match self {
            Command::Twist {
                face,
                layers,
                direction,
            } => {
                f = face.as_mut();
                l = Some(layers);
                d = Some(direction);
            }
            Command::Recenter { face } => f = face.as_mut(),

            Command::HoldSelectFace(face) => f = Some(face),
            Command::HoldSelectLayers(layers) => l = Some(layers),
            Command::HoldSelectPieceType(pieces) => p = Some(pieces),
            Command::ToggleSelectFace(face) => f = Some(face),
            Command::ToggleSelectLayers(layers) => l = Some(layers),
            Command::ToggleSelectPieceType(pieces) => p = Some(pieces),

            _ => (),
        }

        if let Some(f) = f {
            if !puz_type.face_names().contains(&f.as_str()) {
                *f = puz_type.face_names()[0].to_owned();
            }
        }
        if let Some(d) = d {
            if !puz_type.twist_directions().contains(&d.as_str()) {
                *d = puz_type.twist_directions()[0].to_owned();
            }
        }
        if let Some(l) = l {
            l.0 &= (1 << puz_type.layer_count()) - 1;
        }
        if let Some(p) = p {
            p.0 &= (1 << puz_type.piece_types().len()) - 1;
        }
    }
}

/// ID of a face, for use in a keybind.
#[derive(Serialize, Deserialize, Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct FaceId(pub u32);

/// Layer mask, for use in a keybind.
#[derive(Serialize, Deserialize, Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct LayerMask(pub u32);
impl Default for LayerMask {
    fn default() -> Self {
        Self(1)
    }
}

/// Piece type mask, for use in a keybind.
#[derive(Serialize, Deserialize, Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct PieceTypeId(pub u32);
impl Default for PieceTypeId {
    fn default() -> Self {
        Self(u32::MAX)
    }
}
