use hypermath::{APPROX, Float};
use hyperpuzzle_core::TypedIndex;
use hypuz_notation::{AxisLayersInfo, Layer, LayerRange};

/// Cut distances for an axis.
///
/// Distances are measured from the origin and must be sorted from outermost
/// (greatest) to innermost (least).
pub struct CutDistances(pub Vec<Float>);

impl CutDistances {
    /// Returns the number of layers on each axis in the orbit.
    pub fn layer_count(&self) -> usize {
        self.0.len().saturating_sub(1)
    }

    /// Returns the cut distance bounding the outside of each layer, from
    /// outermost to innermost, with an extra `None` at the end.
    fn layer_outside_distances(&self) -> impl Iterator<Item = (Option<Layer>, Float)> {
        Layer::iter(self.layer_count())
            .map(Some)
            .chain([None])
            .zip(self.0.iter().copied())
    }

    /// Returns the layer range for a piece that spans from `min_distance` to
    /// `max_distance` along the axis vector.
    pub fn layer_range_for_distance_range(
        &self,
        max_distance: Float,
        min_distance: Float,
    ) -> Option<LayerRange> {
        // TODO: `None` should represent "not in any layer". blocking the axis
        //       completely is currently unrepresentable
        let (max_layer, _) = self
            .layer_outside_distances()
            .take_while(|(_, d)| APPROX.gt_eq(d, &max_distance))
            .last()?;
        let (min_layer, _) = self
            .layer_outside_distances()
            .take_while(|(_, d)| APPROX.gt(d, &min_distance))
            .last()?;
        Some(LayerRange::new(min_layer?, max_layer?))
    }

    pub fn layers_info(&self) -> AxisLayersInfo {
        AxisLayersInfo {
            max_layer: self.layer_count() as u16,
            allow_negatives: false, // TODO
        }
    }
}
