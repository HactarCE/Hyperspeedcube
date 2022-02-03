//! Common traits used for puzzles.

use cgmath::{Matrix3, Matrix4, SquareMatrix, Vector3, Vector4, Zero};
use std::fmt::Debug;
use std::hash::Hash;
use std::ops::{Index, IndexMut, Mul};
use std::time::Duration;

use super::{Face, LayerMask, Piece, PieceType, PuzzleType, Sticker};
use crate::render::WireframeVertex;

macro_rules! lazy_static_array_methods {
    ( $( $( #[$attr:meta] )* fn $method_name:ident() -> $ret:ty { $($body:tt)* } )* ) => {
        $(
            $( #[$attr] )*
            fn $method_name() -> $ret {
                static STATIC_ARRAY: once_cell::sync::OnceCell<$ret> =
                    once_cell::sync::OnceCell::new();
                STATIC_ARRAY.get_or_init(|| {
                    $($body)*.collect::<Vec<_>>().leak()
                })
            }
        )*
    };
}
macro_rules! lazy_static_generic_array_methods {
    () => {
        lazy_static_array_methods! {
            fn generic_pieces() -> &'static [crate::puzzle::generic::Piece] {
                Self::pieces().iter().map(|&p| p.into())
            }
            fn generic_stickers() -> &'static [crate::puzzle::generic::Sticker] {
                Self::stickers().iter().map(|&s| s.into())
            }
            fn generic_faces() -> &'static [crate::puzzle::generic::Face] {
                Self::faces().iter().map(|&f| f.into())
            }
        }
    };
}

/// Methods for `PuzzleController` that do not depend on puzzle type.
#[enum_dispatch]
pub trait PuzzleControllerTrait {
    /// Returns the puzzle type.
    fn ty(&self) -> PuzzleType;

    /// Advances to the next frame, using the given time delta between this
    /// frame and the last.
    fn advance(&mut self, delta: Duration);
    /// Skips the animations for all twists in the queue.
    fn catch_up(&mut self);

    /// Returns whether there is a move to undo.
    fn has_undo(&self) -> bool;
    /// Returns whether there is a move to redo.
    fn has_redo(&self) -> bool;
    /// Undoes one twist.
    fn undo(&mut self);
    /// Redoes one twist.
    fn redo(&mut self);

    /// Returns whether the puzzle has been modified since the lasts time the
    /// log file was saved.
    fn is_unsaved(&self) -> bool;

    /// Returns the model transform for a piece, based on the current animation
    /// in progress.
    fn model_transform_for_piece(&self, piece: Piece) -> Matrix4<f32>;
    /// Returns whether a sticker is hightlighted.
    fn is_highlighted(&self, sticker: Sticker) -> bool;
    /// Returns the face where the sticker at the given location belongs (i.e.
    /// corresponding to its color).
    fn get_sticker_color(&self, sticker: Sticker) -> Face;
}

/// A twisty puzzle.
///
/// - `puzzle[piece]` is the orientation of the piece at the location given by
///   `piece`.
pub trait PuzzleState:
    'static
    + Debug
    + Default
    + Clone
    + Eq
    + Hash
    + Index<Self::Piece, Output = Self::Orientation>
    + IndexMut<Self::Piece>
{
    /// Location of a piece of the puzzle.
    type Piece: PieceTrait<Self>;
    /// Location of a sticker of the puzzle.
    type Sticker: StickerTrait<Self>;
    /// Location of a face of the puzzle.
    type Face: FaceTrait<Self>;
    /// Twist that can be applied to the puzzle.
    type Twist: TwistTrait<Self>;
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

    /// Number of vertices used to render a single sticker.
    const STICKER_MODEL_VERTEX_COUNT: u16;
    /// Indices of vertices used to render the surface of a single sticker with
    /// the `GL_TRIANGLES` setting.
    const STICKER_MODEL_SURFACE_INDICES: &'static [u16];
    /// Inidices of vertices used to render the outline for a single sticker
    /// with the `GL_LINES` setting.
    const STICKER_MODEL_OUTLINE_INDICES: &'static [u16];

    /// Returns a new solved puzzle in the default orientation.
    fn new() -> Self {
        Self::default()
    }

    /// Swaps two pieces on the puzzle by rotating the first through the
    /// given rotation and rotating the second in the reverse direction.
    fn swap(&mut self, pos1: Self::Piece, pos2: Self::Piece, rot: Self::Orientation) {
        let tmp = self[pos1];
        self[pos1] = rot * self[pos2];
        self[pos2] = rot.rev() * tmp;
    }
    /// Cycles pieces using the given starting piece and the given rotation.
    fn cycle(&mut self, start: Self::Piece, rot: Self::Orientation) {
        let rot = rot.rev();
        let mut prev = start;
        loop {
            let current = rot * prev;
            if current == start {
                break;
            }
            self.swap(current, prev, rot);
            prev = current;
        }
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

    /// Returns a list of pieces in the puzzle.
    fn pieces() -> &'static [Self::Piece];
    /// Returns a list of stickers on the puzzle.
    fn stickers() -> &'static [Self::Sticker];
    /// Returns a list of faces on the puzzle.
    fn faces() -> &'static [Self::Face];

    /// Returns a list of pieces in the puzzle.
    fn generic_pieces() -> &'static [Piece];
    /// Returns a list of stickers in the puzzle.
    fn generic_stickers() -> &'static [Sticker];
    /// Returns a list of faces in the puzzle.
    fn generic_faces() -> &'static [Face];

    /// Returns the short name for each face.
    fn face_symbols() -> &'static [&'static str];
    /// Returns the full name of each face.
    fn face_names() -> &'static [&'static str];
    /// Returns the default color for each face.
    fn default_face_colors() -> &'static [[f32; 3]];

    /// Returns a list of twist directions, not including the identity twist.
    fn twist_direction_names() -> &'static [&'static str];
}

/// Common functionality for puzzle type enumerations.
pub trait PuzzleTypeTrait {
    /// Returns the name of the puzzle.
    fn name(&self) -> &'static str;
    /// Returns the number of dimensions.
    fn ndim(&self) -> usize;
    /// Returns the number of layers.
    fn layer_count(&self) -> usize;

    /// Returns a list of all pieces in the puzzle.
    fn pieces(&self) -> &'static [Piece];
    /// Returns a list of all stickers in the puzzle.
    fn stickers(&self) -> &'static [Sticker];
    /// Returns a list of all faces in the puzzle.
    fn faces(&self) -> &'static [Face];

    /// Returns the names of faces.
    fn face_names(&self) -> &'static [&'static str];
    /// Returns the names of piece types.
    fn piece_type_names(&self) -> &'static [&'static str];
    /// Returns the names of twist directions.
    fn twist_direction_names(&self) -> &'static [&'static str];
    /// Returns the default face colors.
    fn default_face_colors(&self) -> &'static [[f32; 3]];
}

/// Common functionality for all facets (stickers, pieces, and faces).
pub trait FacetTrait: Debug + Copy + Eq + Hash {
    /// Returns the ID of the facet.
    fn id(self) -> usize;
    /// Returns the facet of this type with the given ID, or `None` if the ID is
    /// invalid.
    fn from_id(id: usize) -> Option<Self>;

    /// Returns the 3D-projected center of the facet.
    fn projection_center(self, p: GeometryParams) -> Vector3<f32>;
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
            *MAP.get(&self).expect("invalid facet")
        }

        fn from_id(id: usize) -> Option<Self> {
            $facet_list_expr.get(id).copied()
        }
    };
}

/// The location of a piece in a twisty puzzle.
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

/// The location of a sticker in a twisty puzzle.
pub trait StickerTrait<P: PuzzleState>:
    FacetTrait + Into<P::Sticker> + From<P::Sticker> + Into<Sticker>
{
    /// Returns the piece that this sticker is on.
    fn piece(self) -> P::Piece;
    /// Returns the face that this sticker is on.
    fn face(self) -> P::Face;

    /// Returns the 3D vertices used to render this sticker, or `None` if the
    /// sticker is not visible.
    ///
    /// All vertices should be within the cube from (-1, -1, -1) to (1, 1, 1)
    /// before having `p.transform` applied.
    fn verts(self, p: GeometryParams) -> Option<Vec<WireframeVertex>>;
}

/// A face of a twisty puzzle.
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

/// A twist that can be applied to a twisty puzzle.
pub trait TwistTrait<P: PuzzleState>:
    'static + Debug + Copy + Eq + From<P::Sticker> + Hash
{
    /// Constructs a new twist from a 'twist' command.
    fn from_twist_command(
        face: P::Face,
        direction: &str,
        layers: LayerMask,
    ) -> Result<P::Twist, &'static str>;
    /// Constructs a twist from a 'recenter' command.
    fn from_recenter_command(face: P::Face) -> Result<P::Twist, &'static str>;

    /// Returns the matrix to apply to pieces affected by this twist, given a
    /// time parameter `t` from 0.0 to 1.0. `t=0.0` gives the identity matrix,
    /// `t=1.0` gives the result of the twist, and intermediate values
    /// interpolate.
    fn model_matrix(self, t: f32) -> Matrix4<f32>;

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
}

/// An orientation for a piece of a twisty puzzle, relative to some default.
pub trait OrientationTrait<P: PuzzleState + Hash>:
    Debug + Default + Copy + Eq + Mul<Self, Output = Self> + Mul<P::Piece, Output = P::Piece>
{
    /// Reverses this orientation.
    #[must_use]
    fn rev(self) -> Self;
}

/// Geometry parameters.
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct GeometryParams {
    /// Sticker spacing factor.
    pub sticker_spacing: f32,
    /// Face spacing factor.
    pub face_spacing: f32,
    /// 4D FOV
    pub fov_4d: f32,

    /// Model transformation matrix, including the active twist if applicable.
    pub model_transform: Matrix4<f32>,
    /// View transformation matrix.
    pub view_transform: Matrix3<f32>,

    /// Sticker fill color.
    pub fill_color: [f32; 4],
    /// Outline color.
    pub line_color: [f32; 4],
}
impl Default for GeometryParams {
    fn default() -> Self {
        Self {
            sticker_spacing: 0.2,
            face_spacing: 0.1,
            fov_4d: 0.0,

            model_transform: Matrix4::identity(),
            view_transform: Matrix3::identity(),

            fill_color: [1.0, 1.0, 1.0, 1.0],
            line_color: [0.0, 0.0, 0.0, 1.0],
        }
    }
}
impl GeometryParams {
    /// Computes the sticker scale factor (0.0 to 1.0).
    pub fn sticker_scale(self) -> f32 {
        1.0 - self.sticker_spacing
    }
    /// Computes the sace scale factor (0.0 to 1.0).
    pub fn face_scale(self) -> f32 {
        (1.0 - self.face_spacing) * 3.0 / (2.0 + self.sticker_scale())
    }

    /// Projects a 4D point down to 3D. W coordinates are clipped to the range
    /// from -1 to 1.
    pub fn project_4d(self, point: Vector4<f32>) -> Vector3<f32> {
        // This formula assumes that W is between -1 and 1.
        let w = point.w.clamp(-1.0, 1.0);
        point.truncate() / (1.0 + (1.0 - w) * (self.fov_4d / 2.0).tan())
    }
}

/// Facet of the puzzle.
#[allow(missing_docs)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Facet<P: PuzzleState> {
    Whole,
    Face(P::Face),
    Piece(P::Piece),
    Sticker(P::Sticker),
}
impl<P: PuzzleState> Copy for Facet<P> {}
impl<P: PuzzleState> Facet<P> {
    /// Returns the 3D-projected center of the facet.
    pub fn projection_center(self, p: GeometryParams) -> Vector3<f32> {
        match self {
            Facet::Whole => Vector3::zero(),
            Facet::Face(face) => face.projection_center(p),
            Facet::Piece(piece) => piece.projection_center(p),
            Facet::Sticker(sticker) => sticker.projection_center(p),
        }
    }
}
