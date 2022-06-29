//! Common traits used for puzzles.

use cgmath::{EuclideanSpace, Matrix4, Point3};
use itertools::Itertools;
use std::fmt::Debug;
use std::hash::Hash;
use std::ops::{Index, IndexMut, Mul};

use super::{
    Face, LayerMask, Piece, PieceType, PuzzleType, Sticker, StickerGeometry, StickerGeometryParams,
    Twist, TwistDirection2D, TwistMetric,
};

pub trait PuzzleState: 'static + Debug + Default + Clone + Eq + Hash {
    /// Applies a twist to the puzzle.
    fn twist(&mut self, twist: Twist) {
        let old = self.clone();
        let rot = twist.rotation();
        for piece in twist.pieces() {
            self[rot * piece] = rot * old[piece];
        }
    }

    /// Returns the face where the sticker at the given location belongs
    /// (i.e. corresponding to its color).
    fn get_sticker_color(&self, pos: StickerId) -> FaceId;
    /// Returns whether the puzzle is solved. The default implementation returns
    /// whether all the stickers on each face are the same color.
    fn is_solved(&self) -> bool {
        Self::faces().iter().all(|face| {
            face.stickers()
                .iter()
                .map(|&sticker| self.get_sticker_color(sticker))
                .all_equal()
        })
    }

    /// Returns a list of all pieces in the puzzle.
    fn pieces() -> &'static [Self::Piece];
    /// Returns a list of all stickers on the puzzle.
    fn stickers() -> &'static [Self::Sticker];
    /// Returns a list of all faces on the puzzle.
    fn faces() -> &'static [Self::Face];

    /// Returns a list of all pieces in the puzzle.
    fn generic_pieces() -> &'static [Piece];
    /// Returns a list of all stickers in the puzzle.
    fn generic_stickers() -> &'static [Sticker];
    /// Returns a list of all faces in the puzzle.
    fn generic_faces() -> &'static [Face];

    /// Returns the short names of faces.
    fn face_symbols() -> &'static [&'static str];
    /// Returns the full names of faces.
    fn face_names() -> &'static [&'static str];

    /// Returns a list of all twist axes for the puzzle.
    fn twist_axes() -> &'static [&'static str];
    /// Returns a list of all twist directions for the puzzle.
    fn twist_directions() -> &'static [&'static str];

    /// Returns the short names of twist directions, not including the identity
    /// twist.
    fn twist_direction_symbols() -> &'static [&'static str];
    /// Returns the full names of twist directions, not including the identity
    /// twist.
    fn twist_direction_names() -> &'static [&'static str];
}

pub struct PuzzleState_<F: PuzzleFamily> {
    param: F::Param,
    extra_state: F::ExtraState,
    pieces: Box<[F::Orientation]>,
}

/// Twisty puzzle.
///
/// - `puzzle[piece]` is the orientation of the piece at the location given by
///   `piece`.
pub trait PuzzleState: 'static + Debug + Default + Clone + Eq + Hash
// + Index<Self::Piece, Output = Self::Orientation>
// + IndexMut<Self::Piece>
{
    /// Location of a piece of the puzzle.
    type Piece: PieceTrait<Self>;
    /// Location of a sticker of the puzzle.
    type Sticker: StickerTrait<Self>;
    /// Location of a face of the puzzle.
    type Face: FaceTrait<Self>;

    // /// Axis around which a twist can be performed.
    // type TwistAxis: 'static + Debug + Default + Copy + Eq + Hash;
    // /// Direction that a twist can be performed.
    // ///
    // /// In 3D, this is a rotation in a plane. In 4D, this is a rotation in 3D
    // /// space.
    // type TwistDirection: 'static + Debug + Default + Copy + Eq + Hash;

    type Twist: TwistTrait<Self>; // TODO: remove

    /// Orientation of a puzzle piece, or a rotation that can be applied to an
    /// orientation.
    type Orientation: OrientationTrait<Self>;

    /// User-friendly name for the puzzle.
    const NAME: &'static str;
    /// [`PuzzleType`] enum value.
    const TYPE: PuzzleType;
    /// Number of dimensions of the puzzle.
    const NDIM: usize;
    /// Maximum number of layers that any twist can manipulate. Each layer must
    /// be able to be moved independently.
    const LAYER_COUNT: usize;

    /// Names of piece types.
    const PIECE_TYPE_NAMES: &'static [&'static str];

    /// Number of random moves to fully scramble the puzzle.
    const SCRAMBLE_MOVES_COUNT: usize;

    /// Returns a new solved puzzle in the default orientation.
    fn new() -> Self {
        Self::default()
    }

    /// Applies a twist to the puzzle.
    fn twist(&mut self, twist: Self::Twist) {
        let old = self.clone();
        let rot = twist.rotation();
        for piece in twist.pieces() {
            self[rot * piece] = rot * old[piece];
        }
    }

    /// Returns the face where the sticker at the given location belongs
    /// (i.e. corresponding to its color).
    fn get_sticker_color(&self, pos: Self::Sticker) -> Self::Face;
    /// Returns whether the puzzle is solved. The default implementation returns
    /// whether all the stickers on each face are the same color.
    fn is_solved(&self) -> bool {
        Self::faces().iter().all(|face| {
            face.stickers()
                .iter()
                .map(|&sticker| self.get_sticker_color(sticker))
                .all_equal()
        })
    }

    /// Returns a list of all pieces in the puzzle.
    fn pieces() -> &'static [Self::Piece];
    /// Returns a list of all stickers on the puzzle.
    fn stickers() -> &'static [Self::Sticker];
    /// Returns a list of all faces on the puzzle.
    fn faces() -> &'static [Self::Face];

    /// Returns a list of all pieces in the puzzle.
    fn generic_pieces() -> &'static [Piece];
    /// Returns a list of all stickers in the puzzle.
    fn generic_stickers() -> &'static [Sticker];
    /// Returns a list of all faces in the puzzle.
    fn generic_faces() -> &'static [Face];

    /// Returns the short names of faces.
    fn face_symbols() -> &'static [&'static str];
    /// Returns the full names of faces.
    fn face_names() -> &'static [&'static str];

    /// Returns a list of all twist axes for the puzzle.
    fn twist_axes() -> &'static [&'static str];
    /// Returns a list of all twist directions for the puzzle.
    fn twist_directions() -> &'static [&'static str];

    /// Returns the short names of twist directions, not including the identity
    /// twist.
    fn twist_direction_symbols() -> &'static [&'static str];
    /// Returns the full names of twist directions, not including the identity
    /// twist.
    fn twist_direction_names() -> &'static [&'static str];
}

/// Common functionality for puzzle types.
// pub trait PuzzleTypeTrait: 'static + Debug + Copy + Eq + Hash {
pub trait PuzzleTypeTrait {
    /// Returns the name of the puzzle.
    fn name(self) -> &'static str;
    /// Returns the number of layers.
    fn layer_count(self) -> usize;
    /// Returns the number of moves to fully scramble the puzzle.
    fn scramble_moves_count(self) -> usize;

    /// Returns a list of all pieces in the puzzle.
    fn piece_count(self) -> usize;
    /// Returns a list of all stickers in the puzzle.
    fn sticker_count(self) -> usize;
    /// Returns a list of all faces in the puzzle.
    fn face_count(self) -> usize {
        debug_assert_eq!(self.face_symbols().len(), self.face_names().len());
        self.face_symbols().len()
    }

    /// Returns the short names of faces.
    fn face_symbols(self) -> &'static [&'static str];
    /// Returns the full names of faces.
    fn face_names(self) -> &'static [&'static str];
    /// Returns the names of piece types.
    fn piece_type_names(self) -> &'static [&'static str];

    /// Returns the short names of twist directions, not including the identity
    /// twist.
    fn twist_direction_symbols(self) -> &'static [&'static str];
    /// Returns the full names of twist directions, not including the identity
    /// twist.
    fn twist_direction_names(self) -> &'static [&'static str];
}

/// Common functionality for all facets (stickers, pieces, and faces).
pub trait FacetTrait: Debug + Copy + Eq + Hash {
    /// Returns the ID of the facet.
    fn id(self) -> usize;
    /// Returns the facet of this type with the given ID, or `None` if the ID is
    /// invalid.
    fn from_id(id: usize) -> Option<Self>;

    /// Returns the 3D-projected center of the facet.
    fn projection_center(self, p: StickerGeometryParams) -> Option<Point3<f32>>;
}
macro_rules! impl_facet_trait_id_methods {
    ($facet_type:ty, $facet_list_expr:expr) => {
        fn id(self) -> usize {
            lazy_static! {
                static ref MAP: std::collections::HashMap<$facet_type, usize> = $facet_list_expr
                    .iter()
                    .copied()
                    .enumerate()
                    .map(|(i, facet)| (facet, i))
                    .collect();
            }
            *MAP.get(&self)
                .unwrap_or_else(|| panic!("invalid {}: {:?}", stringify!($facet_type), self))
        }

        fn from_id(id: usize) -> Option<Self> {
            $facet_list_expr.get(id).copied()
        }
    };
}

/// Location of a piece in a twisty puzzle.
pub trait PieceTrait<P: PuzzleState>:
    FacetTrait + Into<P::Piece> + From<P::Piece> + Into<Piece>
{
    /// Returns the piece type of the piece.
    fn piece_type(self) -> PieceType;

    /// Returns the layer of this piece, relative to a face (or `None` if this
    /// does not make sense for the puzzle).
    fn layer(self, face: P::Face) -> Option<usize>;

    /// Returns the number of stickers on this piece (i.e. the length of
    /// `self.stickers()`).
    fn sticker_count(self) -> usize {
        self.stickers().len()
    }
    /// Returns a list of the stickers on this piece.
    fn stickers(self) -> Vec<P::Sticker>;
}

/// Location of a sticker in a twisty puzzle.
pub trait StickerTrait<P: PuzzleState>:
    FacetTrait + Into<P::Sticker> + From<P::Sticker> + Into<Sticker>
{
    /// Returns the piece that this sticker is on.
    fn piece(self) -> P::Piece;
    /// Returns the face that this sticker is on.
    fn face(self) -> P::Face;

    /// Returns the 3D vertex positions used to render this sticker, or `None`
    /// if the sticker is not visible.
    ///
    /// All vertices should be within the cube from (-1, -1, -1) to (1, 1, 1)
    /// before having `p.view_transform` applied.
    fn geometry(self, p: StickerGeometryParams) -> Option<StickerGeometry>;
}

/// Face of a twisty puzzle.
pub trait FaceTrait<P: PuzzleState>:
    'static + FacetTrait + Into<P::Face> + From<P::Face> + Into<Face>
{
    /// Returns the short name for this face.
    fn symbol(self) -> &'static str {
        P::face_symbols()[self.id()]
    }
    /// Returns the full name for this face.
    fn name(self) -> &'static str {
        P::face_names()[self.id()]
    }

    /// Returns a list of all the pieces on this face at one layer.
    fn pieces(self, layer: usize) -> Vec<P::Piece>;
    /// Returns a list of all the stickers on this face.
    fn stickers(self) -> Vec<P::Sticker>;
}

/// Twist that can be applied to a twisty puzzle.
pub trait TwistTrait<P: PuzzleState>:
    'static + Debug + Copy + Eq + Hash + Into<P::Twist> + From<P::Twist> + Into<Twist>
{
    /// Constructs a twist of the outermost layer of a single face.
    fn from_face(face: P::Face, direction: &str) -> Result<P::Twist, &'static str> {
        Self::from_face_with_layers(face, direction, LayerMask::default())
    }
    /// Constructs a twist of a single face.
    fn from_face_with_layers(
        face: P::Face,
        direction: &str,
        layers: LayerMask,
    ) -> Result<P::Twist, &'static str>;
    /// Constructs a twist that recenters a face.
    fn from_face_recenter(face: P::Face) -> Result<P::Twist, &'static str>;
    /// Constructs a twist of a face around a sticker.
    fn from_sticker(
        sticker: P::Sticker,
        direction: TwistDirection2D,
        layers: LayerMask,
    ) -> Result<P::Twist, &'static str>;
    /// Returns a random twist.
    fn from_rng() -> P::Twist;

    /// Returns the matrix to apply to pieces affected by this twist, given a
    /// time parameter `t` from 0.0 to 1.0. `t=0.0` gives the identity matrix,
    /// `t=1.0` gives the result of the twist, and intermediate values
    /// interpolate.
    fn model_transform(self, t: f32) -> Matrix4<f32>;

    /// Returns the orientation that would result from applying this twist to a
    /// piece in the default orientation.
    fn rotation(self) -> P::Orientation;
    /// Returns the reverse of this twist.
    #[must_use]
    fn rev(self) -> Self;
    /// Returns whether a piece is affected by this twist.
    fn affects_piece(self, piece: P::Piece) -> bool;
    /// Returns a list of all the pieces affected by this twist.
    fn pieces(self) -> Vec<P::Piece> {
        P::pieces()
            .iter()
            .copied()
            .filter(|&piece| self.affects_piece(piece))
            .collect()
    }
    /// Returns the destination where a sticker will land after this twist.
    fn destination_sticker(self, sticker: P::Sticker) -> P::Sticker {
        if self.affects_piece(sticker.piece()) {
            self.rotation() * sticker
        } else {
            sticker
        }
    }

    /// Returns whether the two moves are counted as a single move in `metric`.
    fn can_combine(self, previous: Option<Self>, metric: TwistMetric) -> bool;
    /// Returns whether the move is a whole-puzzle rotation, which is not
    /// counted in most turn metrics.
    fn is_whole_puzzle_rotation(self) -> bool;
}

/// Orientation for a piece of a twisty puzzle, relative to some default.
pub trait OrientationTrait<P: PuzzleState + Hash>:
    Debug
    + Default
    + Copy
    + Eq
    + Mul<Self, Output = Self>
    + Mul<P::Piece, Output = P::Piece>
    + Mul<P::Sticker, Output = P::Sticker>
{
    /// Reverses this orientation.
    #[must_use]
    fn rev(self) -> Self;
}
