use smallvec::SmallVec;

use crate::math::{Rotoreflector, Vector, VectorRef};

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

pub trait PuzzleInfo<T> {
    type Output;

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

/// Piece metadata.
#[derive(Debug, Clone, PartialEq)]
pub struct PieceInfo {
    pub stickers: SmallVec<[Sticker; 8]>,
    pub piece_type: PieceType,

    pub points: Vec<Vector>,
}
/// Sticker metadata.
#[derive(Debug, Clone, PartialEq)]
pub struct StickerInfo {
    pub piece: Piece,
    pub color: Facet,

    pub points: Vec<Vector>,
    pub polygons: Vec<Vec<u16>>,
    pub sticker_shrink_origin: Vector,
}
/// Facet metadata.
#[derive(Debug, Clone, PartialEq)]
pub struct FacetInfo {
    pub name: String, // e.g., "Right"
    pub pole: Vector, // face shrink origin
}

/// Twist axis metadata.
#[derive(Debug, Clone, PartialEq)]
pub struct TwistAxisInfo {
    pub symbol: String, // e.g., "R"
    pub opposite: Option<(TwistAxis, Vec<TwistDirection>)>,
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
    /// Returns the opposite twist, if there is one.
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

/// Twist cut metadata.
#[derive(Debug, Clone, PartialEq)]
pub enum TwistCut {
    /// Planar cut perpendicular to the twist axis at a radius from the origin.
    Planar { radius: f32 },
}
impl TwistCut {
    pub fn is_point_above(&self, p: impl VectorRef) -> bool {
        match self {
            TwistCut::Planar { radius } => p.get(0) > *radius,
        }
    }
}

/// Twist direction metadata.
#[derive(Debug, Clone, PartialEq)]
pub struct TwistDirectionInfo {
    pub symbol: String, // "'"
    pub name: String,   // "CCW"
    pub qtm: usize,
    pub rev: TwistDirection,

    pub transform: Rotoreflector,
}
impl AsRef<str> for TwistDirectionInfo {
    fn as_ref(&self) -> &str {
        &self.name
    }
}

/// Piece type metadata.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PieceTypeInfo {
    pub name: String,
}
impl AsRef<str> for PieceTypeInfo {
    fn as_ref(&self) -> &str {
        &self.name
    }
}
impl PieceTypeInfo {
    pub const fn new(name: String) -> Self {
        Self { name }
    }
}
