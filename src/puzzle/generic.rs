use cgmath::Matrix4;
use std::time::Duration;
use thiserror::Error;

use super::{
    rubiks3d, rubiks4d, traits::*, FaceId, LayerMask, PuzzleController, PuzzleType, Rubiks3D,
    Rubiks4D,
};
use crate::render::WireframeVertex;

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

            fn face_names(&self) -> &'static [&'static str];
            fn piece_type_names(&self) -> &'static [&'static str];
            fn twist_direction_names(&self) -> &'static [&'static str];
            fn default_face_colors(&self) -> &'static [[f32; 3]];

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

    /// TODO: refactor/remove this
    pub fn twist_from_command(
        &mut self,
        face_id: FaceId,
        layers: LayerMask,
        direction: &str,
    ) -> Result<(), &'static str> {
        match self {
            Puzzle::Rubiks3D(cube) => cube.twist(rubiks3d::Twist::from_twist_command(
                face_id, direction, layers,
            )?),
            Puzzle::Rubiks4D(cube) => cube.twist(rubiks4d::Twist::from_twist_command(
                face_id, direction, layers,
            )?),
        }
        Ok(())
    }
    /// TODO: refactor/remove this
    pub fn recenter_from_command(&mut self, face_name: &str) -> Result<(), &'static str> {
        let f = FaceId(
            self.ty()
                .face_names()
                .iter()
                .position(|&s| s == face_name)
                .ok_or("invalid face")? as u32,
        );
        match self {
            Puzzle::Rubiks3D(cube) => cube.twist(rubiks3d::Twist::from_recenter_command(f)?),
            Puzzle::Rubiks4D(cube) => cube.twist(rubiks4d::Twist::from_recenter_command(f)?),
        }
        Ok(())
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
generic_facet!(pub struct Sticker, sticker, StickerTrait);
generic_facet!(pub struct Face, face, FaceTrait);

impl Piece {
    delegate_fn_to_puzzle_type! {
        type P = match self.ty();

        /// Returns the piece type ID.
        pub fn piece_type_id(self) -> usize {
            self.try_into::<P>().unwrap().piece_type_id()
        }
        /// Returns the name of the piece type.
        pub fn piece_type_name(self) -> &'static str {
            self.try_into::<P>().unwrap().piece_type_name()
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
        pub fn verts(self, p: GeometryParams) -> Option<Vec<WireframeVertex>> {
            self.try_into::<P>().unwrap().verts(p)
        }
    }
}

impl Face {
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

// pub struct Twist {
//     ty: PuzzleType,
//     todo: (),
// }
// pub struct Orientation {
//     ty: PuzzleType,
//     todo: (),
// }

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
