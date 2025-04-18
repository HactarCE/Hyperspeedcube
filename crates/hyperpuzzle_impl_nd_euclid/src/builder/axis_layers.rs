use eyre::{Result, bail};
use itertools::Itertools;

use hypermath::prelude::*;
use hyperpuzzle_core::prelude::*;

/// Layers of a twist axis.
#[derive(Debug, Clone, Default)]
pub struct AxisLayersBuilder(pub PerLayer<AxisLayerBuilder>);
impl AxisLayersBuilder {
    /// Returns an error if the layers are not monotonic (sorted).
    pub fn ensure_monotonic(&self) -> Result<()> {
        let mut last_depth = Float::INFINITY;
        for layer_info in self.0.iter_values() {
            let AxisLayerBuilder { bottom, top } = layer_info;
            if !(approx_gt_eq(&last_depth, top) && approx_gt(top, bottom)) {
                let depths = self
                    .0
                    .iter_values()
                    .map(|l| (l.top, l.bottom))
                    .collect_vec();
                bail!("axis layers {depths:?} are not sorted from outermost to innermost");
            }
            last_depth = *bottom;
        }
        Ok(())
    }

    /// Validates and finalizes the layer system for an axis.
    pub fn build(&self) -> Result<AxisLayersInfo> {
        // Check that the layer planes are monotonic.
        self.ensure_monotonic()?;

        Ok(AxisLayersInfo(self.0.map_ref(
            |_, &AxisLayerBuilder { bottom, top }| LayerInfo { bottom, top },
        )))
    }
}

/// Layer of a twist axis.
#[derive(Debug, Clone)]
pub struct AxisLayerBuilder {
    /// Position along the axis vector from the origin that bounds the bottom of
    /// the layer. **This may be infinite.**
    pub bottom: Float,
    /// Position along the axis vector from the origin that bounds the top of
    /// the layer. **This may be infinite.**
    pub top: Float,
}
