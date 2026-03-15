use super::{Layer, LayerRange, SignedLayer};

/// Number of layers on an axis and a boolean flag to indicate whether negative
/// layer numbers are allowed.
///
/// By default, the max layer is [`Layer::MAX`] and negative layer numbers are not
/// allowed.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub struct AxisLayersInfo {
    /// Number of layers on the axis, which is equal to the maximum positive
    /// layer.
    pub max_layer: u16,
    /// Whether to allow negative layer numbers to index from the opposite side.
    /// Negative layer numbers are only allowed inside `{}` layer sets. See
    /// [`SignedLayer`] for how negative layer numbers correspond to positive
    /// layer numbers.
    pub allow_negatives: bool,
}

impl Default for AxisLayersInfo {
    fn default() -> Self {
        Self::UNLIMITED
    }
}

impl AxisLayersInfo {
    /// Maximum layer count ([`Layer::MAX`]) and no negative layers.
    pub const UNLIMITED: Self = Self {
        max_layer: Layer::MAX.to_u16(),
        allow_negatives: false,
    };

    /// Resolves a layer, given the number of layers on the axis. See the
    /// [`SignedLayer`] documentation for how this mapping works.
    ///
    /// Returns `None` if the layer is out of range.
    pub fn resolve(self, layer: SignedLayer) -> Option<Layer> {
        if (layer.0.abs().get() as u16) <= self.max_layer {
            if layer.is_positive() {
                Some(Layer(layer.0))
            } else if self.allow_negatives {
                Layer::new(self.max_layer.saturating_sub((-layer.0).get() as u16 - 1))
            } else {
                None
            }
        } else {
            None
        }
    }

    /// Resolves a [`SignedLayer`], returning sentinel values for layers that
    /// are out of range.
    fn resolve_unclamped(self, layer: SignedLayer) -> UnclampedLayer {
        match self.resolve(layer) {
            Some(i) => UnclampedLayer::Concrete(i),
            None if layer.is_positive() => UnclampedLayer::BeyondDeepest,
            None if layer.is_negative() && self.allow_negatives => UnclampedLayer::BeyondShallowest,
            _ => UnclampedLayer::Invalid,
        }
    }

    /// Resolves a range of [`SignedLayer`]s.
    pub(crate) fn resolve_range(self, range: [SignedLayer; 2]) -> Option<LayerRange> {
        let range @ [a, b] = range.map(|l| self.resolve_unclamped(l));
        if a == b && !matches!(a, UnclampedLayer::Concrete(_))
            || a == UnclampedLayer::Invalid
            || b == UnclampedLayer::Invalid
        {
            None
        } else {
            let [a, b] = range.map(|l| match l {
                UnclampedLayer::BeyondShallowest => Layer::SHALLOWEST,
                UnclampedLayer::Concrete(layer) => layer,
                UnclampedLayer::BeyondDeepest => Layer::new_clamped(self.max_layer),
                UnclampedLayer::Invalid => unreachable!(),
            });
            Some(LayerRange::new(a, b))
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum UnclampedLayer {
    BeyondShallowest,
    Concrete(Layer),
    BeyondDeepest,

    Invalid,
}
