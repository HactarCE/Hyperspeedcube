//! Common traits used for puzzles.

use std::fmt::Debug;
use std::hash::Hash;
use std::ops::Mul;

use cgmath::{Matrix4, Vector3};

use super::PuzzleType;

/// A twisty puzzle.
pub trait PuzzleTrait: 'static + Debug + Default + Clone + Eq + Hash {
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

    /// Returns a new solved puzzle in the default orientation.
    fn new() -> Self {
        Self::default()
    }
    /// Returns a reference to the piece at the given location.
    fn get_piece(&self, pos: Self::Piece) -> &Self::Orientation;
    /// Returns a mutable reference to the piece at the given location.
    fn get_piece_mut(&mut self, pos: Self::Piece) -> &mut Self::Orientation;
    /// Returns the face where the sticker at the given location belongs
    /// (i.e. corresponding to its color).
    fn get_sticker(&self, pos: Self::Sticker) -> Self::Face;
    /// Swaps two pieces on the puzzle by rotating the first through the
    /// given rotation and rotating the second in the reverse direction.
    fn swap(&mut self, pos1: Self::Piece, pos2: Self::Piece, rot: Self::Orientation) {
        let tmp = *self.get_piece(pos1);
        *self.get_piece_mut(pos1) = rot * *self.get_piece(pos2);
        *self.get_piece_mut(pos2) = rot.rev() * tmp;
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
        for initial in twist.initial_pieces() {
            self.cycle(initial, twist.rotation())
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
    /// Returns the 3D vertices used to render this sticker, or `None` if the
    /// sticker is not visible.
    ///
    /// All vertices should be within the cube from (-1, -1, -1) to (1, 1, 1).
    fn verts(self, p: GeometryParams, matrix: Matrix4<f32>) -> Option<Vec<Vector3<f32>>>;
    /// Returns an iterator over all the stickers on this puzzle.
    fn iter() -> Box<dyn Iterator<Item = P::Sticker>> {
        Box::new(P::Piece::iter().flat_map(P::Piece::stickers))
    }
}

/// A face of a twisty puzzle.
pub trait FaceTrait<P: PuzzleTrait>: Debug + Copy + Eq + Hash {
    /// The number of faces on this puzzle.
    const COUNT: usize;

    /// Returns a unique number corresponding to this face in the range
    /// 0..Self::COUNT.
    fn idx(self) -> usize;
    /// Returns the color for this face.
    fn color(self) -> [f32; 3];
    /// Returns an iterator over all the stickers on this face.
    fn stickers(self) -> Box<dyn Iterator<Item = P::Sticker> + 'static>;
    /// Returns an iterator over all the faces on this puzzle.
    fn iter() -> Box<dyn Iterator<Item = P::Face>>;
}

/// A twist that can be applied to a twisty puzzle.
pub trait TwistTrait<P: PuzzleTrait>:
    'static + Debug + Copy + Eq + From<P::Sticker> + Hash
{
    /// Returns the orientation that would result from applying this twist
    /// to a piece in the default orientation.
    fn rotation(self) -> P::Orientation;
    /// Returns the reverse of this twist.
    #[must_use]
    fn rev(self) -> Self;
    /// Returns a list of pieces that are the "initial" pieces used to
    /// generate the full list of affected pieces; i.e. self.pieces() is
    /// generated by applying the generator function self.orientation() on
    /// the set of pieces returned by this function.
    ///
    /// Behavior of other methods is undefined if this function returns
    /// multiple pieces that are in the same generated set.
    fn initial_pieces(self) -> Vec<P::Piece>;
    /// Returns an iterator over the pieces affected by this twist.
    fn pieces(self) -> Box<dyn Iterator<Item = P::Piece>> {
        let initial_pieces = self.initial_pieces();
        let rot = self.rotation();
        Box::new(initial_pieces.into_iter().flat_map(move |start| {
            // Include the first piece.
            std::iter::once(start).chain(
                // Keep reorienting that piece ...
                std::iter::successors(Some(rot * start), move |&prev| Some(rot * prev))
                    // ... until we get back where we started.
                    .take_while(move |&x| x != start),
            )
        }))
    }
    /// Returns an iterator over the stickers affected by this twist.
    fn stickers(self) -> Box<dyn Iterator<Item = P::Sticker>> {
        Box::new(self.pieces().flat_map(P::Piece::stickers))
    }
    /// Returns a 4x4 rotation matrix for a portion of this twist, `portion`
    /// ranges from 0.0 to 1.0. 0.0 gives the identity matrix; 1.0 gives the
    /// result of this twist, and intermediate values interpolate.
    fn matrix(self, portion: f32) -> Matrix4<f32>;
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
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct GeometryParams {
    /// Sticker spacing factor.
    pub sticker_spacing: f32,
    /// Face spacing factor.
    pub face_spacing: f32,
    /// 4D FOV
    pub fov_4d: f32,
}
impl Default for GeometryParams {
    fn default() -> Self {
        Self {
            sticker_spacing: 0.2,
            face_spacing: 0.1,
            fov_4d: 0.0,
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
}
