use std::fmt;
use std::sync::Arc;

use super::*;

/// Instance of a puzzle, which tracks the locations of each of its pieces.
pub trait PuzzleState: fmt::Debug + Send + Sync {
    /// Returns the puzzle type.
    fn ty(&self) -> &Arc<PuzzleType>;

    /// Returns a clone of the puzzle state.
    fn clone_boxed(&self) -> Box<dyn PuzzleState>;

    /// Applies a twist to the puzzle. If an error is returned, the puzzle must
    /// remained unchanged.
    fn twist(&mut self, twist: Twist) -> Result<(), &'static str>;
    /// Returns whether a piece is affected by a twist.
    fn is_piece_affected_by_twist(&self, twist: Twist, piece: Piece) -> bool {
        twist.layers[self.layer_from_twist_axis(self.ty().info(twist.transform).axis, piece)]
    }
    /// Returns a list of the pieces affected by a twist.
    fn pieces_affected_by_twist(&self, twist: Twist) -> Vec<Piece> {
        (0..self.ty().pieces.len() as _)
            .map(Piece)
            .filter(|&piece| self.is_piece_affected_by_twist(twist, piece))
            .collect()
    }
    /// Returns the layer of a pieice from a twist axis (i.e., which cuts it is
    /// between).
    ///
    /// TODO: replace with something that allows bandaging/blocking
    fn layer_from_twist_axis(&self, twist_axis: TwistAxis, piece: Piece) -> u8 {
        // // TODO: handle bandaging
        // let axis_info = self.ty().info(twist_axis);

        // let points = &self.ty().info(piece).points;
        // if points.is_empty() {
        //     // TODO: wrong
        //     return 0;
        // }

        // let mut lo = u8::MIN;
        // let mut hi = u8::MAX;
        // for point in points {
        //     let (new_lo, new_hi) = match axis_info.layer_of_point(point) {
        //         // This point is directly on a cut. The piece contains either
        //         // the layer above or the layer below.
        //         PointLayerLocation::OnCut(layer) => (layer, layer + 1),
        //         // The point is between cuts. The piece definitely contains
        //         // this layer.
        //         PointLayerLocation::WithinLayer(layer) => (layer, layer),
        //     };
        //     lo = std::cmp::max(lo, new_lo);
        //     hi = std::cmp::min(hi, new_hi);
        // }
        // if lo != hi {
        //     // TODO: handle bandaging
        //     println!("yikes bandaging");
        // }
        // lo
        todo!()
    }

    /// Returns the N-dimensional transformation to use when rendering a piece
    /// geometry.
    fn piece_transform(&self, p: Piece) -> Matrix;

    /// Returns whether the puzzle is solved.
    ///
    /// TODO: is part solved
    fn is_solved(&self) -> bool;

    /// Appends debug info about a sticker to a string (for development only).
    #[cfg(debug_assertions)]
    fn sticker_debug_info(&self, _s: &mut String, _sticker: Sticker) {}
}
impl Clone for Box<dyn PuzzleState> {
    fn clone(&self) -> Self {
        self.clone_boxed()
    }
}
impl<T: PuzzleState> PuzzleState for Box<T> {
    fn ty(&self) -> &Arc<PuzzleType> {
        (**self).ty()
    }

    fn clone_boxed(&self) -> Box<dyn PuzzleState> {
        (**self).clone_boxed()
    }

    fn twist(&mut self, twist: Twist) -> Result<(), &'static str> {
        (**self).twist(twist)
    }

    fn layer_from_twist_axis(&self, twist_axis: TwistAxis, piece: Piece) -> u8 {
        (**self).layer_from_twist_axis(twist_axis, piece)
    }

    fn piece_transform(&self, p: Piece) -> Matrix {
        (**self).piece_transform(p)
    }

    fn is_solved(&self) -> bool {
        (**self).is_solved()
    }
}
