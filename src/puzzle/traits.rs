//! Common traits used for puzzles.

use cgmath::{Matrix3, SquareMatrix, Vector3, Vector4, Zero};
use std::fmt::Debug;
use std::hash::Hash;
use std::ops::{Index, IndexMut, Mul};

use super::{FaceId, LayerMask, PuzzleType};
use crate::render::WireframeVertex;

/// A twisty puzzle.
///
/// - `puzzle[piece]` is the orientation of the piece at the location given by
///   `piece`.
pub trait PuzzleTrait:
    'static
    + Debug
    + Default
    + Clone
    + Eq
    + Hash
    + Index<Self::Piece, Output = Self::Orientation>
    + IndexMut<Self::Piece>
{
    /// The location of a piece of the puzzle.
    type Piece: PieceTrait<Self>;
    /// The location of a sticker of the puzzle.
    type Sticker: StickerTrait<Self>;
    /// The location of a face of the puzzle.
    type Face: FaceTrait<Self>;
    /// A twist that can be applied to the puzzle.
    type Twist: TwistTrait<Self>;
    /// An orientation for a puzzle piece, or a rotation that can be applied
    /// to an orientation.
    type Orientation: OrientationTrait<Self>;

    /// Number of dimensions of the puzzle.
    const NDIM: usize;
    /// [`PuzzleType`] enum value.
    const TYPE: PuzzleType;
    /// Maximum number of layers that any twist can manipulate. Each layer must
    /// be able to be moved independently.
    const LAYER_COUNT: usize;

    /// Returns a new solved puzzle in the default orientation.
    fn new() -> Self {
        Self::default()
    }
    /// Returns the face where the sticker at the given location belongs
    /// (i.e. corresponding to its color).
    fn get_sticker(&self, pos: Self::Sticker) -> Self::Face;
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
    /// Applies a twist to this puzzle.
    fn twist(&mut self, twist: Self::Twist) {
        let old = self.clone();
        let rot = twist.rotation();
        for piece in twist.pieces() {
            self[rot * piece] = rot * old[piece];
        }
    }
}

/// The location of a piece in a twisty puzzle.
pub trait PieceTrait<P: PuzzleTrait>: Debug + Copy + Eq + Hash {
    /// Returns the number of stickers on this piece (i.e. the length of
    /// self.stickers()).
    fn sticker_count(self) -> usize;
    /// Returns an iterator over all the stickers on this piece.
    fn stickers(self) -> Box<dyn Iterator<Item = P::Sticker> + 'static>;
    /// Returns an iterator over all the pieces in this puzzle.
    fn iter() -> Box<dyn Iterator<Item = Self>>;

    /// Returns the 3D-projected center of the piece.
    fn projection_center(self, p: GeometryParams<P>) -> Vector3<f32>;
}

/// The location of a sticker in a twisty puzzle.
pub trait StickerTrait<P: 'static + PuzzleTrait>: Debug + Copy + Eq + Hash {
    /// The number of vertices used to render a single sticker.
    const VERTEX_COUNT: u16;
    /// The indices of vertices used to render the surface of a single sticker
    /// with the GL_TRIANGLES setting.
    const SURFACE_INDICES: &'static [u16];
    /// The inidices of vertices used to render the outline for a single sticker
    /// with the GL_LINES setting.
    const OUTLINE_INDICES: &'static [u16];

    /// Returns the piece that this sticker is on.
    fn piece(self) -> P::Piece;
    /// Returns the face that this sticker is on.
    fn face(self) -> P::Face;
    /// Returns an iterator over all the stickers on this puzzle.
    fn iter() -> Box<dyn Iterator<Item = P::Sticker>> {
        Box::new(P::Piece::iter().flat_map(P::Piece::stickers))
    }

    /// Returns the 3D-projected center of the sticker.
    fn projection_center(self, p: GeometryParams<P>) -> Vector3<f32>;
    /// Returns the 3D vertices used to render this sticker, or `None` if the
    /// sticker is not visible.
    ///
    /// All vertices should be within the cube from (-1, -1, -1) to (1, 1, 1)
    /// before having `p.transform` applied.
    fn verts(self, p: GeometryParams<P>) -> Option<Vec<WireframeVertex>>;
}

/// A face of a twisty puzzle.
pub trait FaceTrait<P: PuzzleTrait>: Debug + Copy + PartialEq<P::Face> + Eq + Hash {
    /// List of faces on this puzzle.
    const ALL: &'static [P::Face];
    /// Short name for each face.
    const SYMBOLS: &'static [&'static str];
    /// Full name of each face.
    const NAMES: &'static [&'static str];
    /// Default color for each face.
    const DEFAULT_COLORS: &'static [[f32; 3]];

    /// Returns a unique number corresponding to this face in the range
    /// `0..Self::ALL.len()`.
    fn idx(self) -> usize {
        Self::ALL
            .iter()
            .position(|&f| self == f)
            .expect("invalid face")
    }
    /// Returns the short name for this face.
    fn symbol(self) -> &'static str {
        Self::SYMBOLS[self.idx()]
    }
    /// Returns the full name for this face.
    fn name(self) -> &'static str {
        Self::NAMES[self.idx()]
    }
    /// Returns an iterator over all the pieces on this face at one layer.
    fn pieces(self, layer: usize) -> Box<dyn Iterator<Item = P::Piece> + 'static>;
    /// Returns an iterator over all the stickers on this face.
    fn stickers(self) -> Box<dyn Iterator<Item = P::Sticker> + 'static>;

    /// Returns the 3D-projected center of the face.
    fn projection_center(self, p: GeometryParams<P>) -> Vector3<f32>;
}

/// A twist that can be applied to a twisty puzzle.
pub trait TwistTrait<P: PuzzleTrait>:
    'static + Debug + Copy + Eq + From<P::Sticker> + Hash
{
    /// List of twist directions, not including the identity twist.
    const DIRECTIONS: &'static [&'static str];

    /// Constructs a new twist from a keybind.
    fn from_command(face_id: FaceId, direction: &str, layers: LayerMask) -> Result<P::Twist, &str>;

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
        P::Piece::iter()
            .filter(|&piece| self.affects_piece(piece))
            .collect()
    }
}

/// An orientation for a piece of a twisty puzzle, relative to some default.
pub trait OrientationTrait<P: PuzzleTrait + Hash>:
    Debug + Default + Copy + Eq + Mul<Self, Output = Self> + Mul<P::Piece, Output = P::Piece>
{
    /// Reverses this orientation.
    #[must_use]
    fn rev(self) -> Self;
}

/// Geometry parameters.
#[derive(Debug, Clone, PartialEq)]
pub struct GeometryParams<P: PuzzleTrait> {
    /// Sticker spacing factor.
    pub sticker_spacing: f32,
    /// Face spacing factor.
    pub face_spacing: f32,
    /// 4D FOV
    pub fov_4d: f32,

    /// Animation state (twist to animate, and time value from 0.0 to 1.0).
    pub anim: Option<(P::Twist, f32)>,
    /// Model transformation matrix.
    pub transform: Matrix3<f32>,

    /// Sticker fill color.
    pub fill_color: [f32; 4],
    /// Outline color.
    pub line_color: [f32; 4],
}
impl<P: PuzzleTrait> Copy for GeometryParams<P> {}
impl<P: PuzzleTrait> Default for GeometryParams<P> {
    fn default() -> Self {
        Self {
            sticker_spacing: 0.2,
            face_spacing: 0.1,
            fov_4d: 0.0,

            anim: None,
            transform: Matrix3::identity(),

            fill_color: [1.0, 1.0, 1.0, 1.0],
            line_color: [0.0, 0.0, 0.0, 1.0],
        }
    }
}
impl<P: PuzzleTrait> GeometryParams<P> {
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
pub enum Facet<P: PuzzleTrait> {
    Whole,
    Face(P::Face),
    Piece(P::Piece),
    Sticker(P::Sticker),
}
impl<P: PuzzleTrait> Copy for Facet<P> {}
impl<P: PuzzleTrait> Facet<P> {
    /// Returns the 3D-projected center of the facet.
    pub fn projection_center(self, p: GeometryParams<P>) -> Vector3<f32> {
        match self {
            Facet::Whole => Vector3::zero(),
            Facet::Face(face) => face.projection_center(p),
            Facet::Piece(piece) => piece.projection_center(p),
            Facet::Sticker(sticker) => sticker.projection_center(p),
        }
    }
}
