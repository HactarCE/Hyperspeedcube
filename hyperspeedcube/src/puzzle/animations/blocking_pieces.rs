use hyperpuzzle::Piece;
use instant::Instant;

use crate::preferences::InteractionPreferences;

#[derive(Debug, Clone)]
pub struct BlockingPiecesAnimationState {
    /// Pieces that are blocking the last attempted twist.
    blocking_pieces: Vec<Piece>,
    /// Time elapsed since the move was attempted.
    start: Instant,
}
impl Default for BlockingPiecesAnimationState {
    fn default() -> Self {
        Self {
            blocking_pieces: vec![],
            start: Instant::now(),
        }
    }
}
impl BlockingPiecesAnimationState {
    /// Steps the animation forward. Returns whether the puzzle should be
    /// redrawn next frame.
    pub fn proceed(&mut self, prefs: &InteractionPreferences) -> bool {
        let needs_redraw = !self.blocking_pieces.is_empty()
            && self.start.elapsed().as_secs_f32() < prefs.blocking_anim_duration;
        if !needs_redraw {
            self.clear();
        }
        needs_redraw
    }

    pub fn set(&mut self, blocking_pieces: Vec<Piece>) {
        self.blocking_pieces = blocking_pieces;
        self.start = Instant::now();
    }

    pub fn pieces(&self) -> &[Piece] {
        &self.blocking_pieces
    }
    pub fn blocking_amount(&self, prefs: &InteractionPreferences) -> f32 {
        if prefs.blocking_anim_duration == 0.0 {
            0.0
        } else {
            let t = self.start.elapsed().as_secs_f32() / prefs.blocking_anim_duration;
            (2.0 - 2.0 * t).clamp(0.0, 1.0)
        }
    }

    pub fn clear(&mut self) {
        self.blocking_pieces = vec![];
    }
}
