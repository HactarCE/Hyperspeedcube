use cgmath::Matrix4;
use itertools::Itertools;
use std::any::Any;
use std::fmt;
use thiserror::Error;

use super::{traits::*, PuzzleType, Rubiks3D, Rubiks4D, TwistDirection2D, TwistMetric};

macro_rules! delegate_to_inner_puzzle {
    (
        puzzle_types = {[ $($puzzle_type:ident),* ]}
        match_expr = {[ $match_expr:expr ]}
        type_name = {[ $type_name:ident ]}
        var_name = {[ $var_name:ident ]}
        foreach = {[ $foreach:tt ]}
    ) => {
        match $match_expr {
            $(
                Puzzle::$puzzle_type($var_name) => {
                    type $type_name = $puzzle_type;
                    $foreach
                }
            )*
        }
    };
    (
        match $match_expr:expr;
        let $var_name:ident: $type_name:ident => $foreach:expr
    ) => {
        with_puzzle_types! {
            delegate_to_inner_puzzle! {
                puzzle_types = PUZZLE_TYPES
                match_expr = {[ $match_expr ]}
                type_name = {[ $type_name ]}
                var_name = {[ $var_name ]}
                foreach = {[ $foreach ]}
            }
        }
    };
}

/// `Puzzle` of any puzzle type, boxed so that they are always the same size.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Puzzle {
    /// 3D Rubik's cube.
    Rubiks3D(Box<Rubiks3D>),
    /// 4D Rubik's cube.
    Rubiks4D(Box<Rubiks4D>),
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
            fn scramble_moves_count(&self) -> usize;

            fn pieces(&self) -> &'static [Piece];
            fn stickers(&self) -> &'static [Sticker];
            fn faces(&self) -> &'static [Face];

            fn face_symbols(&self) -> &'static [&'static str];
            fn face_names(&self) -> &'static [&'static str];
            fn piece_type_names(&self) -> &'static [&'static str];

            fn twist_direction_symbols(&self) -> &'static [&'static str];
            fn twist_direction_names(&self) -> &'static [&'static str];
        }
    }
}
macro_rules! impl_from_puzzle {
    ( puzzle_types = {[ $($puzzle_type:ident),* ]} ) => {
        $(
            impl From<$puzzle_type> for Puzzle {
                fn from(p: $puzzle_type) -> Self {
                    Self::$puzzle_type(Box::new(p))
                }
            }
        )*
    };
}
with_puzzle_types!(impl_from_puzzle! { puzzle_types = PUZZLE_TYPES });
impl Puzzle {
    /// Creates a new puzzle of a particular type.
    pub fn new(ty: PuzzleType) -> Puzzle {
        delegate_to_puzzle_type! {
            match_expr = {[ ty ]}
            type_name = {[ P ]}
            foreach = {[ Self::from(P::new()) ]}
        }
    }

    /// Returns the type of the puzzle.
    pub fn ty(&self) -> PuzzleType {
        delegate_to_inner_puzzle!(match self; let _p: P => P::TYPE)
    }

    /// Applies a twist to the puzzle.
    pub fn twist(&mut self, twist: Twist) -> Result<(), &'static str> {
        delegate_to_inner_puzzle!(match self; let p: P => {
            Ok(p.twist(twist.of_type::<P>()?))
        })
    }

    /// Returns the face where the sticker at the given location belongs (i.e.
    /// corresponding to its color).
    pub fn get_sticker_color(&self, sticker: Sticker) -> Result<Face, FacetConvertError> {
        delegate_to_inner_puzzle!(match self; let p: P => {
            Ok(p.get_sticker_color(sticker.of_type::<P>()?).into())
        })
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

            /// Converts a generic $name_lower into a $name_lower of a specific
            /// puzzle.
            pub fn of_type<P: PuzzleState>(self) -> Result<P::$name_upper, FacetConvertError> {
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

            /// Converts a generic $name_lower into a $name_lower of a specific
            /// puzzle, panicking if it fails.
            pub fn unwrap<P: PuzzleState>(self) -> P::$name_upper {
                self.of_type::<P>().unwrap()
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
            self.unwrap::<P>().piece_type()
        }

        /// Returns the layer of this piece, relative to a face (or `None` if this
        /// does not make sense for the puzzle).
        pub fn layer(self, face: Face) -> Option<usize> {
            self.unwrap::<P>().layer(face.unwrap::<P>())
        }

        /// Returns the number of stickers on this piece (i.e. the length of
        /// `self.stickers()`).
        pub fn sticker_count(self) -> usize {
            self.unwrap::<P>().sticker_count()
        }
        /// Returns a list of the stickers on this piece.
        pub fn stickers(self) -> Vec<Sticker> {
            self.unwrap::<P>()
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
            self.unwrap::<P>().piece().into()
        }
        /// Returns the face that this sticker is on.
        pub fn face(self) -> Face {
            self.unwrap::<P>().face().into()
        }

        /// Returns the 3D vertices used to render this sticker, or `None` if the
        /// sticker is not visible.
        ///
        /// All vertices should be within the cube from (-1, -1, -1) to (1, 1, 1)
        /// before having `p.transform` applied.
        pub fn geometry(self, p: StickerGeometryParams) -> Option<StickerGeometry> {
            self.unwrap::<P>().geometry(p)
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

        /// Returns the short name of the face.
        pub fn symbol(self) -> &'static str {
            P::face_symbols()[self.id()]
        }
        /// Returns the full name of the face.
        pub fn name(self) -> &'static str {
            P::face_names()[self.id()]
        }

        /// Returns a list of all the pieces on this face at one layer.
        pub fn pieces(self, layer: usize) -> Vec<Piece> {
            self.unwrap::<P>()
                .pieces(layer)
                .into_iter()
                .map(|f| f.into())
                .collect()
        }
        /// Returns a list of all the stickers on this face.
        pub fn stickers(self) -> Vec<Sticker> {
            self.unwrap::<P>()
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

    /// Returns the short name of the twist direction.
    pub fn symbol(self) -> &'static str {
        self.ty.twist_direction_symbols()[self.id]
    }
    /// Returns the full name of the twist direction.
    pub fn name(self) -> &'static str {
        self.ty.twist_direction_names()[self.id]
    }
}

/// Twist of a puzzle.
#[derive(Debug)]
pub struct Twist {
    pub(super) ty: PuzzleType,
    pub(super) inner: Box<dyn Any>,
}
impl Clone for Twist {
    delegate_fn_to_puzzle_type! {
        type P = match self.ty;

        fn clone(&self) -> Self {
            Self {
                ty: self.ty,
                inner: Box::new(self.unwrap::<P>()),
            }
        }
    }
}
impl Eq for Twist {}
impl PartialEq for Twist {
    fn eq(&self, other: &Self) -> bool {
        self.ty() == other.ty()
            && delegate_to_puzzle_type! {
                match_expr = {[ self.ty() ]}
                type_name = {[ P ]}
                foreach = {[ self.unwrap::<P>() == other.unwrap::<P>() ]}
            }
    }
}
macro_rules! generic_twist_from {
    (
        puzzle_types = {[ $($puzzle_type:ident),* ]}
    ) => {
        $(
            impl From<<$puzzle_type as PuzzleState>::Twist> for Twist {
                fn from(twist: <$puzzle_type as PuzzleState>::Twist) -> Self {
                    Self {
                        ty: PuzzleType::$puzzle_type,
                        inner: Box::new(twist),
                    }
                }
            }
        )*
    };
}
with_puzzle_types! { generic_twist_from! { puzzle_types = PUZZLE_TYPES } }
impl fmt::Display for Twist {
    delegate_fn_to_puzzle_type! {
        type P = match self.ty;

        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            self.unwrap::<P>().fmt(f)
        }
    }
}
impl Twist {
    /// Converts a generic twist into a twist of a specific puzzle.
    pub fn of_type<P: PuzzleState>(&self) -> Result<P::Twist, &'static str> {
        if self.ty != P::TYPE {
            return Err("puzzle type mismatch");
        }
        self.inner
            .downcast_ref::<P::Twist>()
            .cloned()
            .ok_or("internal error when downcasting twist")
    }
    /// Converts a generic twist into a twist of a specific puzzle, panicking if
    /// it fails.
    pub fn unwrap<P: PuzzleState>(&self) -> P::Twist {
        self.of_type::<P>().unwrap()
    }

    /// Constructs a twist of the outermost layer of a single face.
    pub fn from_face(face: Face, direction: &str) -> Result<Twist, &'static str> {
        delegate_to_puzzle_type! {
            match_expr = {[ face.ty() ]}
            type_name = {[ P ]}
            foreach = {[
                Ok(<P as PuzzleState>::Twist::from_face(face.unwrap::<P>(), direction)?.into())
            ]}
        }
    }
    /// Constructs a twist of a single face.
    pub fn from_face_with_layers(
        face: Face,
        direction: &str,
        layers: LayerMask,
    ) -> Result<Twist, &'static str> {
        delegate_to_puzzle_type! {
            match_expr = {[ face.ty() ]}
            type_name = {[ P ]}
            foreach = {[
                Ok(<P as PuzzleState>::Twist::from_face_with_layers(face.unwrap::<P>(), direction, layers)?.into())
            ]}
        }
    }
    /// Constructs a twist that recenters a face.
    pub fn from_face_recenter(face: Face) -> Result<Twist, &'static str> {
        delegate_to_puzzle_type! {
            match_expr = {[ face.ty() ]}
            type_name = {[ P ]}
            foreach = {[
                Ok(<P as PuzzleState>::Twist::from_face_recenter(face.unwrap::<P>())?.into())
            ]}
        }
    }
    /// Constructs a twist of a face around a sticker.
    pub fn from_sticker(
        sticker: Sticker,
        direction: TwistDirection2D,
        layers: LayerMask,
    ) -> Result<Twist, &'static str> {
        delegate_to_puzzle_type! {
            match_expr = {[ sticker.ty() ]}
            type_name = {[ P ]}
            foreach = {[
                Ok(<P as PuzzleState>::Twist::from_sticker(sticker.unwrap::<P>(), direction, layers)?.into())
            ]}
        }
    }
    /// Returns a random twist.
    pub fn from_rng(ty: PuzzleType) -> Twist {
        delegate_to_puzzle_type! {
            match_expr = {[ ty ]}
            type_name = {[ P ]}
            foreach = {[
                <P as PuzzleState>::Twist::from_rng().into()
            ]}
        }
    }

    /// Returns the matrix to apply to pieces affected by this twist, given a
    /// time parameter `t` from 0.0 to 1.0. `t=0.0` gives the identity matrix,
    /// `t=1.0` gives the result of the twist, and intermediate values
    /// interpolate.
    pub fn model_transform(&self, t: f32) -> Matrix4<f32> {
        delegate_to_puzzle_type! {
            match_expr = {[ self.ty() ]}
            type_name = {[ P ]}
            foreach = {[
                self.unwrap::<P>().model_transform(t)
            ]}
        }
    }

    delegate_fn_to_puzzle_type! {
        type P = match self.ty;

        /// Returns the reverse of this twist.
        #[must_use]
        pub fn rev(&self) -> Self {
            self.unwrap::<P>().rev().into()
        }
        /// Returns whether a piece is affected by this twist.
        pub fn affects_piece(&self, piece: Piece) -> bool {
            self.unwrap::<P>().affects_piece(piece.unwrap::<P>())
        }
        /// Returns the destination where a sticker will land after this twist.
        pub fn destination_sticker(&self, sticker: Sticker) -> Sticker {
            self.unwrap::<P>().destination_sticker(sticker.unwrap::<P>()).into()
        }

        /// Returns whether the two moves are counted as a single move in
        /// `metric`.
        pub fn can_combine(&self, previous: Option<&Self>, metric: TwistMetric) -> bool {
            self.unwrap::<P>()
                .can_combine(previous.and_then(|t| t.of_type::<P>().ok()), metric)
        }
    }

    /// Returns a list of all the pieces affected by this twist.
    pub fn pieces(&self) -> Vec<Piece> {
        self.ty
            .pieces()
            .iter()
            .copied()
            .filter(|&piece| self.affects_piece(piece))
            .collect()
    }

    /// Returns the puzzle type.
    pub fn ty(&self) -> PuzzleType {
        self.ty
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
impl std::ops::Index<usize> for LayerMask {
    type Output = bool;

    fn index(&self, index: usize) -> &Self::Output {
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

    pub(crate) fn validate<P: PuzzleState>(self) -> Result<(), &'static str> {
        if self.0 > 0 || self.0 < 1 << P::LAYER_COUNT {
            Ok(())
        } else {
            Err("invalid layer mask")
        }
    }
    pub(crate) const fn all<P: PuzzleState>() -> Self {
        Self((1 << P::LAYER_COUNT as u32) - 1)
    }
    pub(crate) fn is_all(self, ty: PuzzleType) -> bool {
        self.0 == (1 << ty.layer_count() as u32) - 1
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
