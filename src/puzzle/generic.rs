use cgmath::{Matrix4, Point3};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use smallvec::{smallvec, SmallVec};
use std::any::Any;
use std::fmt;
use thiserror::Error;

use super::{rubiks_3d, traits::*, Rubiks3D, StickerGeometry, StickerGeometryParams, TwistMetric};

/// Enumeration of all puzzle types.
#[derive(Serialize, Deserialize, Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum PuzzleTypeEnum {
    /// 3D Rubik's cube.
    Rubiks3D { layer_count: u8 },
    // /// 4D Rubik's cube.
    // Rubiks4D { layer_count: u8 },
}
#[delegate_to_methods]
#[delegate(PuzzleType, target_ref = "as_dyn_type")]
impl PuzzleTypeEnum {
    fn as_dyn_type(&self) -> &dyn PuzzleType {
        match *self {
            PuzzleTypeEnum::Rubiks3D { layer_count } => rubiks_3d::puzzle_type(layer_count),
            // PuzzleTypeEnum::Rubiks4D { .. } => todo!("4D type"),
        }
    }
}
impl Default for PuzzleTypeEnum {
    fn default() -> Self {
        Self::Rubiks3D { layer_count: 3 }
    }
}
impl fmt::Display for PuzzleTypeEnum {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}
impl AsRef<str> for PuzzleTypeEnum {
    fn as_ref(&self) -> &str {
        self.name()
    }
}

#[derive(Serialize, Deserialize, Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum PieceType {} // TODO: remove

// TODO: do not allow ser/de on these

#[derive(Serialize, Deserialize, Debug, Default, Copy, Clone, PartialEq, Eq, Hash)]
pub struct Piece(pub u16);
#[derive(Serialize, Deserialize, Debug, Default, Copy, Clone, PartialEq, Eq, Hash)]
pub struct Sticker(pub u16);
#[derive(Serialize, Deserialize, Debug, Default, Copy, Clone, PartialEq, Eq, Hash)]
pub struct Face(pub u8);
#[derive(Serialize, Deserialize, Debug, Default, Copy, Clone, PartialEq, Eq, Hash)]
pub struct TwistAxis(pub u8);
#[derive(Serialize, Deserialize, Debug, Default, Copy, Clone, PartialEq, Eq, Hash)]
pub struct TwistDirection(pub u8);

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub struct PieceInfo {
    pub stickers: SmallVec<[Sticker; 8]>,
}
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct StickerInfo {
    pub piece: Piece,
    pub face: Face, //  color of the sticker
}
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct FaceInfo {
    pub symbol: &'static str, // "R"
    pub name: &'static str,   // "Right"
}
impl FaceInfo {
    pub const fn new(symbol: &'static str, name: &'static str) -> Self {
        Self { symbol, name }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct TwistAxisInfo {
    pub name: &'static str, // "U"
}
impl TwistAxisInfo {
    pub(super) fn list_from_faces(face_list: &[FaceInfo]) -> Vec<Self> {
        face_list.iter().map(|f| Self { name: f.symbol }).collect()
    }
}
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct TwistDirectionInfo {
    pub name: &'static str, // "CW"
}
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct Twist {
    pub axis: TwistAxis,
    pub direction: TwistDirection,
    pub layer_mask: LayerMask,
}

/// Puzzle of any type.
#[enum_dispatch(PuzzleType, PuzzleState)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Puzzle {
    /// 3D Rubik's cube.
    Rubiks3D(Rubiks3D),
    // /// 4D Rubik's cube.
    // Rubiks34(Box<Rubiks34>),
}
impl Default for Puzzle {
    fn default() -> Self {
        Self::new(PuzzleTypeEnum::default())
    }
}
impl Puzzle {
    /// Creates a new puzzle of a particular type.
    pub fn new(ty: PuzzleTypeEnum) -> Puzzle {
        match ty {
            PuzzleTypeEnum::Rubiks3D { layer_count } => {
                Puzzle::Rubiks3D(Rubiks3D::new(layer_count))
            } // PuzzleTypeEnum::Rubiks4D { .. } => todo!("construct 4D rubiks cube"),
        }
    }
}

// /// Facet of the puzzle.
// #[allow(missing_docs)]
// #[derive(Debug, Copy, Clone, PartialEq, Eq)]
// pub enum Facet {
//     Whole,
//     Face(Face),
//     Piece(Piece),
//     Sticker(Sticker),
// }
// impl Facet {
//     /// Returns the 3D-projected center of the facet.
//     pub fn projection_center(self, p: StickerGeometryParams) -> Option<Point3<f32>> {
//         match self {
//             Facet::Whole => Some(Point3::origin()),
//             Facet::Face(face) => face.projection_center(p),
//             Facet::Piece(piece) => piece.projection_center(p),
//             Facet::Sticker(sticker) => sticker.projection_center(p),
//         }
//     }
// }

/// Layer mask, for use in a keybind.
#[derive(Serialize, Deserialize, Debug, Copy, Clone, PartialEq, Eq, Hash)]
#[serde(transparent)]
pub struct LayerMask(pub u32);
impl Default for LayerMask {
    fn default() -> Self {
        Self(1)
    }
}
impl std::ops::Index<u8> for LayerMask {
    type Output = bool;

    fn index(&self, index: u8) -> &Self::Output {
        match self.0 & (1 << index) {
            0 => &false,
            _ => &true,
        }
    }
}
impl LayerMask {
    pub(crate) fn is_default(self) -> bool {
        self == Self::default()
    }
    pub(crate) fn short_description(self) -> String {
        // Just give up if there's more than 9 layers.
        (0..9)
            .filter(|&i| self[i])
            .map(|i| (i as u8 + '1' as u8) as char)
            .collect()
    }
    pub(crate) fn long_description(self) -> String {
        (0..32).filter(|&i| self[i]).map(|i| i + 1).join(", ")
    }
}

/// Selection of twist axis and layers.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct TwistSelection {
    /// Bitmask of selected twist axes.
    pub axis_mask: u32,
    /// Bitmask of selected layers.
    pub layer_mask: u32,
}
impl Default for TwistSelection {
    fn default() -> Self {
        Self {
            axis_mask: 0_u32,
            layer_mask: 1_u32,
        }
    }
}
impl std::ops::BitOr for TwistSelection {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self {
            axis_mask: self.axis_mask | rhs.axis_mask,
            layer_mask: self.layer_mask | rhs.layer_mask,
        }
    }
}
impl std::ops::BitXorAssign for TwistSelection {
    fn bitxor_assign(&mut self, rhs: Self) {
        self.axis_mask ^= rhs.axis_mask;
        self.layer_mask ^= rhs.layer_mask;
    }
}
impl TwistSelection {
    /// Returns the layer mask if any layers are selected, or the default layer
    /// mask (outermost layer only) otherwise.
    pub fn layer_mask_or_default(self) -> LayerMask {
        if self.layer_mask != 0_u32 {
            LayerMask(self.layer_mask)
        } else {
            LayerMask::default()
        }
    }

    /// Returns whether the twist selection includes a particular sticker.
    pub fn has_sticker(self, puzzle: &dyn PuzzleState, sticker: Sticker) -> bool {
        let piece = puzzle.info(sticker).piece;

        // Filter by twist_axis and layer.
        let layer_mask = self.layer_mask_or_default();
        (0..puzzle.twist_axes().len() as _)
            .filter(|i| (self.axis_mask >> i) & 1 != 0)
            .map(TwistAxis)
            .map(|twist_axis| puzzle.layer_from_twist_axis(twist_axis, piece))
            .all(|layer| layer_mask[layer])
    }
}
