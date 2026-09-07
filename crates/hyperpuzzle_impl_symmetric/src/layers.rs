use std::fmt;

use hypermath::prelude::pga::Motor;
use hypermath::prelude::*;
use hyperpuzzle_core::prelude::*;
use itertools::Itertools;

/// Layer depths for each axis of a puzzle.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct PuzzleLayerDepths(pub PerLayer<LayerDepths>);
impl fmt::Display for PuzzleLayerDepths {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let top_bottom_pairs = self
            .0
            .iter_values()
            .map(|l| (l.top, l.bottom))
            .collect_vec();
        write!(f, "{top_bottom_pairs:?}")
    }
}
impl PuzzleLayerDepths {
    /// Returns whether the layer list is empty.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Returns the number of layers.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns whether a layer range is contiguous on the puzzle.
    pub fn is_range_contiguous(&self, range: LayerRange) -> bool {
        range
            .into_iter()
            .tuple_windows()
            .all(|(higher, lower)| APPROX.eq(self.0[higher].bottom, self.0[lower].top))
    }

    /// Returns the smallest contiguous layer range that contains two floats, or
    /// `None` if there is none.
    pub fn contiguous_range(&self, lo: Float, hi: Float) -> Option<LayerMask> {
        let bottom_layer = self.0.find(|_, l| APPROX.lt_eq(l.bottom, lo))?;
        let top_layer = self.0.rfind(|_, l| APPROX.gt_eq(l.top, hi))?;
        Some(LayerRange::new(top_layer, bottom_layer))
            .filter(|&range| self.is_range_contiguous(range))
            .map(LayerMask::from_range)
    }
}

/// Top & bottom depths for one axis.
#[derive(Debug, Clone, PartialEq)]
pub struct LayerDepths {
    /// Position along the axis vector from the origin that bounds the top of
    /// the layer. **This may be infinite.**
    pub top: Float,
    /// Position along the axis vector from the origin that bounds the bottom of
    /// the layer. **This may be infinite.**
    pub bottom: Float,
}
impl TransformByMotor for LayerDepths {
    fn transform_by(&self, _m: &Motor) -> Self {
        Self {
            top: self.top,
            bottom: self.bottom,
        }
    }
}
