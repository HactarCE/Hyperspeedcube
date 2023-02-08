use anyhow::{Context, Result};
use smallvec::SmallVec;

use super::LayerMask;
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

/// Piece ID.
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, Hash)]
pub struct Piece(pub u16);
/// Sticker ID.
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Sticker(pub u16);
/// Facet ID.
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Facet(pub u16);
/// Twist axis ID.
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TwistAxis(pub u16);
/// Twist transform ID.
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TwistTransform(pub u32);
/// Piece type ID.
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PieceType(pub u8);

macro_rules! impl_conversions {
    ($($ty:ty),* $(,)?) => { $(
        impl tinyset::Fits64 for $ty {
            unsafe fn from_u64(x: u64) -> Self { Self(x as _) }
            fn to_u64(self) -> u64 { self.0 as u64 }
        }
        impl From<$ty> for usize {
            fn from(x: $ty) -> Self { x.0 as usize }
        }
    )* };
}
impl_conversions!(Piece, Sticker, Facet, TwistAxis, TwistTransform, PieceType);

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
impl FacetInfo {
    /// Returns the normal vector (normalized pole). Returns an error if the
    /// facet intersects the origin.
    pub fn normal(&self) -> Result<Vector> {
        self.pole.normalize().context("facet intersects origin")
    }
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
    pub fn layer_of_point(&self, point: impl VectorRef) -> PointLayerLocation {
        let point_radius = self.normal.dot(point);
        match self.cuts.binary_search_by(|cut| cut.cmp(point_radius)) {
            Ok(layer) => PointLayerLocation::OnCut(layer as _),
            Err(layer) => PointLayerLocation::WithinLayer(layer as _),
        }
    }
}

/// Twist axis layer containing a point.
pub enum PointLayerLocation {
    /// The point is on the cut between this layer and the next one.
    OnCut(u8),
    /// The point is within this layer.
    WithinLayer(u8),
}

/// Twist cut info.
#[derive(Debug, Clone, PartialEq)]
pub enum TwistCut {
    /// Planar cut perpendicular to the twist axis.
    Planar {
        /// Distance from the orgin.
        radius: f32,
    },
}
impl TwistCut {
    /// Compares a point's radius to the twist cut, returning `Less` if it is
    /// below the cut, `Greater` if it is above the cut, or `Equal` if it is
    /// approximately on the cut.
    pub(super) fn cmp(&self, point_radius: f32) -> std::cmp::Ordering {
        match self {
            TwistCut::Planar { radius } => approx_cmp(&point_radius, &radius),
        }
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
    pub transform: Rotoreflector,

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
