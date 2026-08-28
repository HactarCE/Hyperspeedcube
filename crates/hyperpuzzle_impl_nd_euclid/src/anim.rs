use std::any::Any;

use hypermath::pga;
use hyperpuzzle_core::prelude::*;

/// Animation for an N-dimensional Euclidean puzzle.
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
    fn clone_dyn(&self) -> BoxDynPuzzleAnimation
    where
        Self: Sized,
    {
        self.clone().into()
    }

    fn simultaneous(&self, other: &dyn PuzzleAnimation) -> Option<BoxDynPuzzleAnimation> {
        let other = (other as &dyn Any).downcast_ref::<Self>()?;
        if self.pieces == other.pieces {
            Some(
                Self {
                    pieces: self.pieces.clone(),
                    initial_transform: self.initial_transform.clone(),
                    final_transform: &other.final_transform * &self.final_transform,
                }
                .into(),
            )
        } else {
            None
        }
    }
}
