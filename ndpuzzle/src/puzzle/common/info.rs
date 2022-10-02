use smallvec::SmallVec;

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
/// Face ID.
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Face(pub u8);
/// Twist axis ID.
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TwistAxis(pub u8);
/// Twist direction ID.
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TwistDirection(pub u8);
/// Piece type ID.
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PieceType(pub u8);

/// Piece metadata.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PieceInfo {
    pub stickers: SmallVec<[Sticker; 8]>,
    pub piece_type: PieceType,
}
/// Sticker metadata.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct StickerInfo {
    pub piece: Piece,
    pub color: Face,
}
/// Face metadata.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FaceInfo {
    pub name: String, // e.g., "Right"
}
impl FaceInfo {
    pub const fn new(name: String) -> Self {
        Self { name }
    }
}

/// Twist axis metadata.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TwistAxisInfo {
    pub symbol: String, // e.g., "R"
    pub layer_count: u8,
    pub opposite: Option<(TwistAxis, Vec<TwistDirection>)>,
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
}

/// Twist direction metadata.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TwistDirectionInfo {
    pub symbol: String, // "'"
    pub name: String,   // "CCW"
    pub qtm: usize,
    pub rev: TwistDirection,
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
