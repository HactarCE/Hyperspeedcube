use std::fmt;
use std::str::FromStr;

use hypermath::collections::{GenericMask, GenericVec};
use hypermath::pga::Motor;
use hypermath::prelude::*;
use itertools::Itertools;
use serde::de::Error;
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;
use tinyset::Set64;

use super::LayerMask;
use crate::Rgb;

hypermath::idx_struct! {
    /// ID of a **piece**, which is rigid component of the puzzle that moves
    /// together.
    #[derive(Serialize, Deserialize)]
    pub struct Piece(pub u32);

    /// ID of a **sticker**, which is a facet of a **piece** having a single
    /// color and belonging to a single **facet**.
    #[derive(Serialize, Deserialize)]
    pub struct Sticker(pub u32);

    /// ID of a **twist gizmo face**, which is a single face that can be clicked
    /// to twist the puzzle.
    #[derive(Serialize, Deserialize)]
    pub struct GizmoFace(pub u16);

    /// ID of a **surface**, which is an external facet of the puzzle shared by
    /// one or more **stickers**.
    #[derive(Serialize, Deserialize)]
    pub struct Surface(pub u16);

    /// ID of a **color** that appears on stickers.
    #[derive(Serialize, Deserialize)]
    pub struct Color(pub u16);

    /// ID of a **twist axis**, an organizational unit containing several
    /// **twists**.
    #[derive(Serialize, Deserialize)]
    pub struct Axis(pub u16);

    /// ID of a **twist**, which is a single move that can be applied to the
    /// puzzle.
    #[derive(Serialize, Deserialize)]
    pub struct Twist(pub u32);

    /// ID of a **layer**, which is a region of the puzzle for each axis that may be twisted by a move on that axis.
    #[derive(Serialize, Deserialize)]
    pub struct Layer(pub u8);

    /// ID of a **piece type**, a subset of the **pieces** of the puzzle.
    #[derive(Serialize, Deserialize)]
    pub struct PieceType(pub u16);
}

impl Surface {
    /// Surface ID for pieces that are not on a external surface, such as
    /// internals.
    pub const NONE: Surface = Surface::MAX;
}
impl Color {
    /// Color ID for internals.
    pub const INTERNAL: Color = Color::MAX;
}

/// List containing a value per piece.
pub type PerPiece<T> = GenericVec<Piece, T>;
/// List containing a value per sticker.
pub type PerSticker<T> = GenericVec<Sticker, T>;
/// List containing a value per twist gizmo face.
pub type PerGizmoFace<T> = GenericVec<GizmoFace, T>;
/// List containing a value per surface.
pub type PerSurface<T> = GenericVec<Surface, T>;
/// List containing a value per color.
pub type PerColor<T> = GenericVec<Color, T>;
/// List containing a value per twist axis.
pub type PerAxis<T> = GenericVec<Axis, T>;
/// List containing a value per twist.
pub type PerTwist<T> = GenericVec<Twist, T>;
/// List containing a value per layer.
pub type PerLayer<T> = GenericVec<Layer, T>;
/// List containing a value per piece type.
pub type PerPieceType<T> = GenericVec<PieceType, T>;

/// Sparse set of pieces in a puzzle.
pub type PieceSet = Set64<Piece>;
/// Sparse set of stickers in a puzzle.
pub type StickerSet = Set64<Sticker>;
/// Sparse set of surfaces in a puzzle.
pub type SurfaceSet = Set64<Surface>;
/// Sparse set of colors in a puzzle.
pub type ColorSet = Set64<Color>;

/// Dense set of pieces in a puzzle.
pub type PieceMask = GenericMask<Piece>;
/// Dense set of piece types in a puzzle.
pub type PieceTypeMask = GenericMask<PieceType>;

/// Piece info.
#[derive(Debug, PartialEq)]
pub struct PieceInfo {
    /// Unordered list of stickers on the piece.
    pub stickers: SmallVec<[Sticker; 8]>,
    /// Piece type.
    pub piece_type: PieceType,
}

/// Sticker info.
#[derive(Debug, PartialEq)]
pub struct StickerInfo {
    /// Piece that the sticker is part of.
    pub piece: Piece,
    /// Color on the sticker.
    pub color: Color,
}

/// Twist axis info.
#[derive(Debug)]
pub struct AxisInfo {
    /// Name for the twist axis. (e.g, "U", "R", etc.)
    pub name: String,
    /// Additional names for the twist axis.
    pub aliases: Vec<String>,
    /// Layer.
    pub layers: AxisLayers,
    /// Opposite axis, which has a reversed layer list.
    pub opposite: Option<Axis>,
}
impl AsRef<str> for AxisInfo {
    fn as_ref(&self) -> &str {
        &self.name
    }
}

/// Layers of an axis.
#[derive(Debug, Default, PartialEq)]
pub struct AxisLayers(pub PerLayer<LayerInfo>);
impl fmt::Display for AxisLayers {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let top_bottom_pairs = self
            .0
            .iter_values()
            .map(|l| (l.top, l.bottom))
            .collect_vec();
        write!(f, "{top_bottom_pairs:?}")
    }
}
impl AxisLayers {
    /// Returns whether the layer list is empty.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Returns the number of layers.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns whether a layer range is contiguous on the puzzle.
    pub fn is_range_contiguous(&self, range: std::ops::RangeInclusive<u8>) -> bool {
        range
            .map(Layer)
            .tuple_windows()
            .all(|(higher, lower)| approx_eq(&self.0[higher].bottom, &self.0[lower].top))
    }

    /// Returns the smallest contiguous layer range that contains two floats, or
    /// `None` if there is none.
    pub fn contiguous_range(&self, lo: Float, hi: Float) -> Option<LayerMask> {
        let bottom_layer = self.0.find(|_, l| approx_lt_eq(&l.bottom, &lo))?.0;
        let top_layer = self.0.rfind(|_, l| approx_gt_eq(&l.top, &hi))?.0;

        // Ensure layers are contiguous
        if !self.is_range_contiguous(top_layer..=bottom_layer) {
            return None;
        }

        Some(LayerMask::from(top_layer..=bottom_layer))
    }
}

/// Layer info.
#[derive(Debug, PartialEq)]
pub struct LayerInfo {
    /// Position along the axis vector from the origin that bounds the bottom of
    /// the layer. **This may be infinite.**
    pub bottom: Float,
    /// Position along the axis vector from the origin that bounds the top of
    /// the layer. **This may be infinite.**
    pub top: Float,
}
impl TransformByMotor for LayerInfo {
    fn transform_by(&self, _m: &Motor) -> Self {
        Self {
            bottom: self.bottom,
            top: self.top,
        }
    }
}

/// Twist info.
#[derive(Debug)]
pub struct TwistInfo {
    /// Human-friendly name for the twist. (e.g., "U2", "R'", etc.)
    pub name: String,
    /// Additional names for the twist.
    pub aliases: Vec<String>,

    /// Value of this twist in quarter turn metric.
    pub qtm: usize,

    /// Twist axis to use to determine which pieces are moved by the twist.
    pub axis: Axis,

    /// Opposite twist. With a reversed layer mask, this applies the
    /// same transformation to the same pieces. For example, R and L' are
    /// opposite twists on a 3x3x3.
    pub opposite: Option<Twist>,

    /// Reverse twist, which undoes this one.
    pub reverse: Twist,

    /// Whether to include this twist in scrambles.
    pub include_in_scrambles: bool,
}
impl AsRef<str> for TwistInfo {
    fn as_ref(&self) -> &str {
        &self.name
    }
}
impl TwistInfo {
    /// Returns the lexicographically first name or alias.
    pub fn min_name(&self) -> &String {
        match self.aliases.iter().min() {
            Some(min_alias) => std::cmp::min(min_alias, &self.name),
            None => &self.name,
        }
    }
}

/// Piece type info.
#[derive(Debug, PartialEq, Eq, Hash)]
pub struct PieceTypeInfo {
    #[allow(clippy::doc_markdown)]
    /// Name for the piece type. (e.g., "center/oblique_1_2/left")
    pub name: String,
    /// User-friendly display name for the piece type. (e.g., "Oblique (1, 2)
    /// (left)")
    ///
    /// This is also stored in the piece type hierarchy.
    pub display: String,
}
impl AsRef<str> for PieceTypeInfo {
    fn as_ref(&self) -> &str {
        &self.name
    }
}

/// Color info.
#[derive(Debug, PartialEq, Eq, Hash)]
pub struct ColorInfo {
    /// Name for the color. (e.g., "U", "R", etc.)
    pub name: String,
    /// Additional names for the color.
    pub aliases: Vec<String>,
    /// Display name for the color. (e.g., "Up", "Right", etc.)
    pub display: String,
}

/// Default color for a puzzle color.
#[allow(missing_docs)]
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
pub enum DefaultColor {
    /// Unknown default color.
    #[default]
    Unknown,
    /// Specific hexcode, such as `#ff00ff` or `#f0f`.
    HexCode { rgb: Rgb },
    /// Single named color.
    Single { name: String },
    /// Color from a named set.
    Set { set_name: String, index: usize },
    /// Color from a gradient.
    Gradient {
        gradient_name: String,
        index: usize,
        total: usize,
    },
}
impl FromStr for DefaultColor {
    type Err = eyre::Report;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();

        if s.starts_with('#') {
            return Ok(Self::HexCode { rgb: s.parse()? });
        }

        let name = s.to_string();

        // IIFE to mimic try_block
        Ok((|| {
            let (set_name, index) = s.strip_suffix(']')?.split_once('[')?;
            let set_name = set_name.trim().to_string();
            if let Some((index, total)) = index.split_once('/') {
                let gradient_name = set_name;
                let index = index.trim().parse::<usize>().ok()?.saturating_sub(1); // 1-indexed
                let total = total.trim().parse::<usize>().ok()?;
                Some(Self::Gradient {
                    gradient_name,
                    index,
                    total,
                })
            } else {
                let index = index.trim().parse::<usize>().ok()?.saturating_sub(1); // 1-indexed
                Some(Self::Set { set_name, index })
            }
        })()
        .unwrap_or(Self::Single { name }))
    }
}
impl fmt::Display for DefaultColor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DefaultColor::Unknown => write!(f, "(unknown)"),
            DefaultColor::HexCode { rgb } => write!(f, "{rgb}"),
            DefaultColor::Single { name } => write!(f, "{name}"),
            DefaultColor::Set { set_name, index } => write!(f, "{set_name} [{}]", index + 1), /* 1-indexed */
            DefaultColor::Gradient {
                gradient_name,
                index: numerator,
                total: denominator,
            } => write!(
                f,
                "{gradient_name} [{}/{}]",
                numerator.saturating_add(1),
                denominator,
            ), /* 1-indexed */
        }
    }
}
impl Serialize for DefaultColor {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.to_string().serialize(serializer)
    }
}
impl<'de> Deserialize<'de> for DefaultColor {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        String::deserialize(deserializer)?
            .parse()
            .map_err(D::Error::custom)
    }
}
