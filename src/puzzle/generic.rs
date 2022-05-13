use cgmath::Matrix4;
use std::fmt;
use std::time::Duration;
use thiserror::Error;

use super::{traits::*, PuzzleController, PuzzleType, Rubiks3D, Rubiks4D, TwistMetric};
use crate::preferences::Preferences;
use crate::render::RgbaVertex;

/// A PuzzleController of any puzzle type.
#[derive(PartialEq, Eq)]
#[enum_dispatch(PuzzleControllerTrait)]
pub enum Puzzle {
    /// A 3D Rubik's cube.
    Rubiks3D(PuzzleController<Rubiks3D>),
    /// A 4D Rubik's cube.
    Rubiks4D(PuzzleController<Rubiks4D>),
}
impl Default for Puzzle {
    fn default() -> Self {
        Self::new(PuzzleType::default())
    }
}
impl PuzzleTypeTrait for Puzzle {
    delegate! {
        to self.ty() {
            fn name(&self) -> &'static str;
            fn ndim(&self) -> usize;
            fn layer_count(&self) -> usize;

            fn pieces(&self) -> &'static [Piece];
            fn stickers(&self) -> &'static [Sticker];
            fn faces(&self) -> &'static [Face];

            fn face_symbols(&self) -> &'static [&'static str];
            fn face_names(&self) -> &'static [&'static str];
            fn piece_type_names(&self) -> &'static [&'static str];
            fn twist_direction_names(&self) -> &'static [&'static str];
        }
    }
}
impl Puzzle {
    /// Creates a new puzzle of this type.
    pub fn new(ty: PuzzleType) -> Puzzle {
        delegate_to_puzzle_type! {
            match_expr = {[ ty ]}
            type_name = {[ P ]}
            foreach = {[ Self::from(PuzzleController::<P>::new()) ]}
        }
    }
}

macro_rules! generic_facet {
    (pub struct $name_upper:ident, $name_lower:ident, $trait_name:ident) => {
        /// $name_upper of a puzzle of any type.
        #[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
        pub struct $name_upper {
            ty: PuzzleType,
            id: usize,
        }
        impl $name_upper {
            /// Returns the puzzle type.
            pub fn ty(self) -> PuzzleType {
                self.ty
            }
            /// Returns the $name_lower ID.
            pub fn id(self) -> usize {
                self.id
            }

            /// Converts a generic $name_lower into a $name_lower of a specific puzzle.
            pub fn try_into<P: PuzzleState>(self) -> Result<P::$name_upper, FacetConvertError> {
                if self.ty != P::TYPE {
                    return Err(FacetConvertError::PuzzleTypeMismatch {
                        expected: P::TYPE,
                        actual: self.ty,
                    });
                }
                P::$name_upper::from_id(self.id).ok_or(FacetConvertError::InvalidId {
                    puzzle: self.ty,
                    facet: FacetType::$name_upper,
                    id: self.id,
                })
            }
        }
        with_puzzle_types! {
            generic_facet_from! {
                puzzle_types = PUZZLE_TYPES
                impl From<$trait_name> for $name_upper {}
            }
        }
    };
}
macro_rules! generic_facet_from {
    (
        puzzle_types = {[ $($puzzle_type:ident),* ]}
        impl From<$trait_name:ident> for $name_upper:ident {}
    ) => {
        $(
            impl From<<$puzzle_type as PuzzleState>::$name_upper> for $name_upper {
                fn from(facet: <$puzzle_type as PuzzleState>::$name_upper) -> Self {
                    Self {
                        ty: PuzzleType::$puzzle_type,
                        id: facet.id(),
                    }
                }
            }
        )*
    };
}

generic_facet!(pub struct Piece, piece, PieceTrait);
impl Piece {
    delegate_fn_to_puzzle_type! {
        type P = match self.ty();

        /// Returns the piece type.
        pub fn piece_type(self) -> PieceType {
            self.try_into::<P>().unwrap().piece_type()
        }

        /// Returns the layer of this piece, relative to a face (or `None` if this
        /// does not make sense for the puzzle).
        pub fn layer(self, face: Face) -> Option<usize> {
            self.try_into::<P>().unwrap().layer(face.try_into::<P>().unwrap())
        }

        /// Returns the number of stickers on this piece (i.e. the length of
        /// `self.stickers()`).
        pub fn sticker_count(self) -> usize {
            self.try_into::<P>().unwrap().sticker_count()
        }
        /// Returns a list of the stickers on this piece.
        pub fn stickers(self) -> Vec<Sticker> {
            self.try_into::<P>()
                .unwrap()
                .stickers()
                .into_iter()
                .map(|s| s.into())
                .collect()
        }
    }
}

generic_facet!(pub struct Sticker, sticker, StickerTrait);
impl Sticker {
    delegate_fn_to_puzzle_type! {
        type P = match self.ty();

        /// Returns the piece that this sticker is on.
        pub fn piece(self) -> Piece {
            self.try_into::<P>().unwrap().piece().into()
        }
        /// Returns the face that this sticker is on.
        pub fn face(self) -> Face {
            self.try_into::<P>().unwrap().face().into()
        }

        /// Returns the 3D vertices used to render this sticker, or `None` if the
        /// sticker is not visible.
        ///
        /// All vertices should be within the cube from (-1, -1, -1) to (1, 1, 1)
        /// before having `p.transform` applied.
        pub fn verts(self, p: StickerGeometryParams) -> Option<Vec<RgbaVertex>> {
            self.try_into::<P>().unwrap().verts(p)
        }
    }
}

generic_facet!(pub struct Face, face, FaceTrait);
impl fmt::Display for Face {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}
impl AsRef<str> for Face {
    fn as_ref(&self) -> &str {
        self.name()
    }
}
impl Face {
    /// Returns the face with a particular name, or a default if none exists.
    pub fn from_name(ty: PuzzleType, name: &str) -> Self {
        Self::try_from_name(ty, name).unwrap_or_else(|| ty.faces()[0])
    }
    /// Returns the face with a particular name, if one exists.
    pub fn try_from_name(ty: PuzzleType, name: &str) -> Option<Self> {
        ty.face_names()
            .iter()
            .position(|&s| s == name)
            .map(|id| Self { ty, id })
    }

    delegate_fn_to_puzzle_type! {
        type P = match self.ty();

        /// Returns the short name for this face.
        pub fn symbol(self) -> &'static str {
            P::face_symbols()[self.id()]
        }
        /// Returns the full name for this face.
        pub fn name(self) -> &'static str {
            P::face_names()[self.id()]
        }

        /// Returns a list of all the pieces on this face at one layer.
        pub fn pieces(self, layer: usize) -> Vec<Piece> {
            self.try_into::<P>()
                .unwrap()
                .pieces(layer)
                .into_iter()
                .map(|f| f.into())
                .collect()
        }
        /// Returns a list of all the stickers on this face.
        pub fn stickers(self) -> Vec<Sticker> {
            self.try_into::<P>()
                .unwrap()
                .stickers()
                .into_iter()
                .map(|s| s.into())
                .collect()
        }
    }
}

/// Piece type, for use in a keybind.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct PieceType {
    pub(super) ty: PuzzleType,
    pub(super) id: usize,
}
impl fmt::Display for PieceType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}
impl PieceType {
    /// Returns a default piece type.
    pub fn default(ty: PuzzleType) -> Self {
        Self { ty, id: 0 }
    }

    /// Returns an iterator over all piece types of a puzzle.
    pub fn iter(ty: PuzzleType) -> impl Iterator<Item = Self> {
        (0..ty.piece_type_names().len()).map(move |id| Self { ty, id })
    }
    /// Returns the piece type with a particular name, or a default if none
    /// exists.
    pub fn from_name(ty: PuzzleType, name: &str) -> Self {
        Self::try_from_name(ty, name).unwrap_or_else(|| Self::default(ty))
    }
    /// Returns the piece type with a particular name, if one exists.
    pub fn try_from_name(ty: PuzzleType, name: &str) -> Option<Self> {
        ty.piece_type_names()
            .iter()
            .position(|&s| s == name)
            .map(move |id| Self { ty, id })
    }

    /// Returns the puzzle type.
    pub fn ty(self) -> PuzzleType {
        self.ty
    }
    /// Returns the piece type ID.
    pub fn id(self) -> usize {
        self.id
    }

    /// Returns the name of the piece type.
    pub fn name(self) -> &'static str {
        self.ty.piece_type_names()[self.id]
    }
}

/// Twist direction, for use in a keybind.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct TwistDirection {
    pub(super) ty: PuzzleType,
    pub(super) id: usize,
}
impl fmt::Display for TwistDirection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}
impl AsRef<str> for TwistDirection {
    fn as_ref(&self) -> &str {
        self.name()
    }
}
impl TwistDirection {
    /// Returns a default twist direction.
    pub fn default(ty: PuzzleType) -> Self {
        Self { ty, id: 0 }
    }

    /// Returns an iterator over all twist directions of a puzzle.
    pub fn iter(ty: PuzzleType) -> impl Iterator<Item = Self> {
        (0..ty.twist_direction_names().len()).map(move |id| Self { ty, id })
    }
    /// Returns the twist direction with a particular name, or a default if none
    /// exists.
    pub fn from_name(ty: PuzzleType, name: &str) -> Self {
        Self::try_from_name(ty, name).unwrap_or_else(|| Self::default(ty))
    }
    /// Returns the twist direction with a particular name, if one exists.
    pub fn try_from_name(ty: PuzzleType, name: &str) -> Option<Self> {
        ty.twist_direction_names()
            .iter()
            .position(|&s| s == name)
            .map(|id| Self { ty, id })
    }

    /// Returns the puzzle type.
    pub fn ty(self) -> PuzzleType {
        self.ty
    }
    /// Returns the twist direction ID.
    pub fn id(self) -> usize {
        self.id
    }

    /// Returns the name of the twist direction.
    pub fn name(self) -> &'static str {
        self.ty.twist_direction_names()[self.id]
    }
}

/// Layer mask, for use in a keybind.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct LayerMask(pub u32);
impl Default for LayerMask {
    fn default() -> Self {
        Self(1)
    }
}

#[derive(Error, Debug)]
#[allow(missing_docs)]
pub enum FacetConvertError {
    #[error("puzzle type mismatch (expected {expected:?}, got {actual:?})")]
    PuzzleTypeMismatch {
        expected: PuzzleType,
        actual: PuzzleType,
    },
    #[error("invalid ID ({puzzle:?} has no {facet:?} with ID {id:?})")]
    InvalidId {
        puzzle: PuzzleType,
        facet: FacetType,
        id: usize,
    },
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
#[allow(missing_docs)]
pub enum FacetType {
    Piece,
    Sticker,
    Face,
}

/// Selection of faces, layers, and piece types.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct Selection {
    /// Bitmask of selected faces.
    pub face_mask: u32,
    /// Bitmask of selected layers.
    pub layer_mask: u32,
    /// Bitmask of selected piece types.
    pub piece_type_mask: u32,
}
impl Default for Selection {
    fn default() -> Self {
        Self {
            face_mask: 0,
            layer_mask: 0,
            piece_type_mask: u32::MAX,
        }
    }
}
impl std::ops::BitOr for Selection {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self {
            face_mask: self.face_mask | rhs.face_mask,
            layer_mask: self.layer_mask | rhs.layer_mask,
            piece_type_mask: self.piece_type_mask | rhs.piece_type_mask,
        }
    }
}
impl std::ops::BitXorAssign for Selection {
    fn bitxor_assign(&mut self, rhs: Self) {
        self.face_mask ^= rhs.face_mask;
        self.layer_mask ^= rhs.layer_mask;
        self.piece_type_mask ^= rhs.piece_type_mask;
    }
}
impl Selection {
    /// Returns the face selected if exactly one face is selected; otherwise
    /// returns `None`.
    pub fn exactly_one_face(&self, puzzle_type: PuzzleType) -> Option<Face> {
        if self.face_mask.count_ones() == 1 {
            let face_id = self.face_mask.trailing_zeros() as usize; // index of first `1` bit
            puzzle_type.faces().get(face_id).copied()
        } else {
            None
        }
    }
    /// Returns the layer mask if any layers are selected, or a default layer
    /// mask (generally just one layer) otherwise.
    pub fn layer_mask_or_default(self, default: LayerMask) -> LayerMask {
        if self.layer_mask != 0 {
            LayerMask(self.layer_mask)
        } else {
            default
        }
    }

    /// Returns whether the selection includes a particular sticker.
    pub fn has_sticker(self, sticker: Sticker) -> bool {
        let puzzle_type = sticker.ty();
        let piece = sticker.piece();

        // Filter by piece type.
        if self.piece_type_mask & (1 << piece.piece_type().id()) == 0 {
            return false;
        }

        // Filter by face and layer.
        let layer_mask = self.layer_mask_or_default(LayerMask::default());
        for (i, &face) in puzzle_type.faces().iter().enumerate() {
            if (self.face_mask >> i) & 1 != 0 {
                if let Some(l) = piece.layer(face) {
                    if (layer_mask.0 >> l) & 1 != 0 {
                        continue;
                    }
                }
                return false;
            }
        }

        true
    }
}
