use hypermath::pga;

use crate::{BoxDynPuzzleAnimation, PerPiece, PieceMask, PuzzleAnimation, PuzzleStateRenderData};

// /// Data that needs to be uploaded to the GPU before rendering.
// pub enum PuzzleTypeGpuBuffers {
//     /// N-dimensional Euclidean mesh data.
//     Hypershape(),
//     /// No GPU data
//     None,
// }

/// Puzzle render data for an N-dimensional Euclidean puzzle.
pub struct NdEuclidPuzzleStateRenderData {
    /// Transform for each piece.
    pub piece_transforms: PerPiece<pga::Motor>,
}
impl PuzzleStateRenderData for NdEuclidPuzzleStateRenderData {}

#[derive(Debug, Clone)]
pub struct NdEuclidPuzzleAnimation {
    /// Set of pieces affected by the animation.
    pub pieces: PieceMask,
    /// Initial transform of the pieces (identity, unless the move was inputted
    /// using a mouse drag).
    pub initial_transform: pga::Motor,
    /// Final transform for the pieces.
    pub final_transform: pga::Motor,
}
impl PuzzleAnimation for NdEuclidPuzzleAnimation {
    fn dyn_clone(&self) -> BoxDynPuzzleAnimation
    where
        Self: Sized,
    {
        self.clone().into()
    }
}
