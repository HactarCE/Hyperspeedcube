use std::{range::Range, sync::Arc};

use hypermath::{Float, RangeMap, WhichSide, collections::NanError};
use hypuz_notation::{AxisLayersInfo, Layer, LayerMask, LayerRange};

/// Ranges of cut depths along an axis corresponding to layers.
///
/// This type is cheap to clone.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct LayerMap {
    map: Arc<RangeMap<Option<Layer>>>,
    layers_info: AxisLayersInfo,
    covers_whole_space: bool,
}
impl LayerMap {
    pub fn new(map: Arc<RangeMap<Option<Layer>>>, allow_negatives: bool) -> Self {
        let max_layer = map
            .iter()
            .filter_map(|(_range, layer)| *layer)
            .map(|l| l.to_u16())
            .max()
            .unwrap_or(0);

        let covers_whole_space = map.iter().all(|(_range, value)| value.is_some());

        Self {
            map,
            layers_info: AxisLayersInfo {
                max_layer,
                allow_negatives,
            },
            covers_whole_space,
        }
    }

    pub fn intersecting_layers(
        &self,
        range: impl Into<Range<Float>>,
    ) -> Result<IntersectedLayers, NanError> {
        Ok(self.map.get_range(range)?.copied().collect())
    }

    pub fn info(&self) -> AxisLayersInfo {
        self.layers_info
    }

    pub fn covers_whole_space(&self) -> bool {
        self.covers_whole_space
    }
}

/// Set of layers that a piece is touching, including "outside any layer."
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
pub struct IntersectedLayers {
    pub layers: LayerMask,
    pub outside_any_layer: bool,
}

impl IntersectedLayers {
    pub fn insert(&mut self, opt_layer: Option<Layer>) {
        match opt_layer {
            Some(l) => self.layers.insert(l),
            None => self.outside_any_layer = true,
        }
    }
}

impl FromIterator<Option<Layer>> for IntersectedLayers {
    fn from_iter<T: IntoIterator<Item = Option<Layer>>>(iter: T) -> Self {
        let mut ret = Self::default();
        for opt_layer in iter {
            ret.insert(opt_layer);
        }
        ret
    }
}

impl IntersectedLayers {
    /// Returns whether the intersected layers set of a piece is on the inside
    /// or outside of a twist, or whether it is blocking.
    ///
    /// If the piece has no volume, then [`WhichSide::Flush`] is returned.
    pub fn which_side(&self, twist_layers: &LayerMask) -> WhichSide {
        let is_any_inside = !(twist_layers & &self.layers).is_empty();
        let is_any_outside = self.outside_any_layer || {
            let max_layer = std::cmp::max(self.layers.capacity(), twist_layers.capacity());
            let max_layer_range = LayerRange::new(Layer::SHALLOWEST, max_layer);
            let mut inverted_twist_layers = twist_layers.clone();
            inverted_twist_layers.invert_range(max_layer_range);
            !(inverted_twist_layers & &self.layers).is_empty()
        };
        WhichSide::from_bools(is_any_inside, is_any_outside)
    }
}
