use smallvec::SmallVec;

use super::{LayerMask, TwistCut};
use crate::collections::{GenericVec, IndexNewtype};
use crate::math::*;

macro_rules! impl_puzzle_info_trait {
    (for $t:ty { fn info($thing:ty) -> &$thing_info:ty { $($tok:tt)* } }) => {
        impl $crate::puzzle::PuzzleInfo<$thing> for $t {
            type Output = $thing_info;

            fn info(&self, thing: $thing) -> &$thing_info {
                &self $($tok)* [thing.0 as usize]
            }
        }
    };
}

/// Trait for retrieving information about puzzle elements that is independent
/// of state.
pub trait PuzzleInfo<T> {
    /// Type containing info about the element.
    type Output;

    /// Returns state-independent information about a puzzle element.
    fn info(&self, thing: T) -> &Self::Output;
}

idx_struct! {
    /// Piece ID.
    pub struct Piece(pub u16);
    /// Sticker ID.
    pub struct Sticker(pub u16);
    /// Facet ID.
    pub struct Facet(pub u16);
    /// Twist axis ID.
    pub struct TwistAxis(pub u16);
    /// Twist transform ID.
    pub struct TwistTransform(pub u32);
    /// Piece type ID.
    pub struct PieceType(pub u8);
}

impl Facet {
    /// Facet ID for internals.
    pub const INTERNAL: Facet = Facet::MAX;
}

/// List containing a value per piece.
pub type PerPiece<T> = GenericVec<Piece, T>;
/// List containing a value per sticker.
pub type PerSticker<T> = GenericVec<Sticker, T>;
/// List containing a value per facet.
pub type PerFacet<T> = GenericVec<Facet, T>;
/// List containing a value per twist axis.
pub type PerTwistAxis<T> = GenericVec<TwistAxis, T>;
/// List containing a value per twist transform.
pub type PerTwistTransform<T> = GenericVec<TwistTransform, T>;
/// List containing a value per piece type.
pub type PerPieceType<T> = GenericVec<PieceType, T>;

/// Piece info.
#[derive(Debug, Clone, PartialEq)]
pub struct PieceInfo {
    /// Unordered list of stickers in the piece.
    pub stickers: SmallVec<[Sticker; 8]>,
    /// Piece type.
    pub piece_type: PieceType,

    /// Unordered list of vertices that comprise the piece.
    pub points: Vec<Vector>,
}

/// Sticker info.
#[derive(Debug, Clone, PartialEq)]
pub struct StickerInfo {
    /// Piece that the sticker is part of.
    pub piece: Piece,
    /// Facet whose color is on the sticker.
    pub color: Facet,

    /// List of vertices that comprise the sticker.
    pub points: Vec<Vector>,
    /// Vector along which to shrink each point.
    pub shrink_vectors: Vec<Vector>,
    /// List of polygons for rendering the sticker.
    ///
    /// Each polygon is a list of indices into `points`.
    pub polygons: Vec<SmallVec<[u16; 8]>>,
}

/// Facet info.
#[derive(Debug, Clone, PartialEq)]
pub struct FacetInfo {
    /// Human-friendly name for the facet. (e.g., "Up", "Right", etc.)
    pub name: String,
    /// Point on the facet that is closest to the origin. This is a scalar
    /// multiple of the facet's normal vector.
    pub pole: Vector,
    /// Name of default color.
    pub default_color: Option<String>,
}

/// Twist axis info.
#[derive(Debug, Clone, PartialEq)]
pub struct TwistAxisInfo {
    /// Human-friendly name for the twist axis. (e.g, "U", "R", etc.)
    pub name: String,

    /// Vector that is perpendicular to cuts along the axis.
    pub normal: Vector,
    /// Cuts along the axis.
    pub cuts: Vec<TwistCut>,

    /// Transforms that can be applied on this axis, sorted lexicographically by
    /// name.
    pub transforms: Vec<TwistTransform>,

    /// Opposite twist axis, if there is one.
    pub opposite: Option<TwistAxis>,
}
impl AsRef<str> for TwistAxisInfo {
    fn as_ref(&self) -> &str {
        &self.name
    }
}
impl TwistAxisInfo {
    /// Returns the number of layers on the twist axis.
    pub fn layer_count(&self) -> u8 {
        self.cuts.len() as u8 + 1
    }
    /// Returns the maximum layer mask for the twist axis.
    pub fn all_layers(&self) -> LayerMask {
        LayerMask((1 << self.layer_count()) - 1)
    }
}

/// Twist transform info.
#[derive(Debug, Clone, PartialEq)]
pub struct TwistTransformInfo {
    /// Human-friendly name for the twist. (e.g., "U2", "R'", etc.)
    pub name: String,

    /// Value of this twist in quarter turn metric.
    pub qtm: usize,

    /// Twist axis to use to determine which pieces are moved by the twist.
    pub axis: TwistAxis,
    /// Transforation to apply to pieces.
    pub transform: cga::Isometry,

    /// Opposite twist transform. With a reversed layer mask, this applies the
    /// same transformation to the same pieces. For example, R and L' are
    /// opposite twists on a 3x3x3.
    pub opposite: Option<TwistTransform>,

    /// Reverse twist transform, which undoes this one.
    pub reverse: TwistTransform,
}
impl AsRef<str> for TwistTransformInfo {
    fn as_ref(&self) -> &str {
        &self.name
    }
}

/// Piece type info.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PieceTypeInfo {
    /// TODO: remove and replace with piece type hierarchy
    pub name: String,
}
impl AsRef<str> for PieceTypeInfo {
    fn as_ref(&self) -> &str {
        &self.name
    }
}
