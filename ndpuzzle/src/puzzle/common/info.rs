use anyhow::{Context, Result};
use smallvec::SmallVec;

use crate::math::{abs_diff_cmp, Rotoreflector, Vector, VectorRef};

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
pub struct Facet(pub u8); // TODO: expand to u16
/// Twist axis ID.
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TwistAxis(pub u8); // TODO: expand to u16
/// Twist direction ID.
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TwistDirection(pub u8); // TODO: expand to u16
/// Piece type ID.
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PieceType(pub u8);

macro_rules! impl_fits64 {
    ($($ty:ty),* $(,)?) => { $(
        impl tinyset::Fits64 for $ty {
            unsafe fn from_u64(x: u64) -> Self { Self(x as _) }
            fn to_u64(self) -> u64 { self.0 as u64 }
        }
    )* };
}
impl_fits64!(Piece, Sticker, Facet, TwistAxis, TwistDirection, PieceType);

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
    pub symbol: String,
    /// Opposite twist axis, and each corresponding twist direction on that
    /// opposite axis.
    pub opposite: Option<(TwistAxis, Vec<TwistDirection>)>,
    /// Cuts along the axis.
    pub cuts: Vec<TwistCut>,

    /// Transformation from puzzle space to the local space of the twist axis.
    /// Applying this transformation moves the X axis to the this axis's normal.
    pub reference_frame: Rotoreflector,
}
impl AsRef<str> for TwistAxisInfo {
    fn as_ref(&self) -> &str {
        &self.symbol
    }
}
impl TwistAxisInfo {
    /// Returns the opposite twist axis, if there is one.
    pub fn opposite_axis(&self) -> Option<TwistAxis> {
        self.opposite.as_ref().map(|(axis, _)| *axis)
    }
    /// Returns the twist on the opposite axis, if there is one.
    pub fn opposite_twist(&self, dir: TwistDirection) -> Option<(TwistAxis, TwistDirection)> {
        self.opposite
            .as_ref()
            .and_then(|(axis, dirs)| Some((*axis, *dirs.get(dir.0 as usize)?)))
    }
    /// Returns the number of layers on the twist axis.
    pub fn layer_count(&self) -> u8 {
        self.cuts.len() as u8 + 1
    }
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
    /// Compares a transformed point to the twist cut, returning `Less` if it is
    /// below the cut, `Greater` if it is above the cut, or `Equal` if it is
    /// approximately on the cut.
    pub(super) fn cmp(&self, point: impl VectorRef) -> std::cmp::Ordering {
        match self {
            TwistCut::Planar { radius } => abs_diff_cmp(&point.get(0), &radius),
        }
    }
}

/// Twist direction info.
#[derive(Debug, Clone, PartialEq)]
pub struct TwistDirectionInfo {
    /// TODO: remove
    pub symbol: String,
    /// Human-friendly name for the twist direction. (e.g., "CW", "x", etc.).
    pub name: String,
    /// Number of QTM twists required to have the same effect as this twist.
    pub qtm: usize,
    /// Opposite twist direction.
    pub rev: TwistDirection,

    /// Transformation this twist applies to pieces.
    pub transform: Rotoreflector,
}
impl AsRef<str> for TwistDirectionInfo {
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
