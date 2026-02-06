use std::fmt;

use super::Layer;

/// Non-empty inclusive [`Layer`] range.
///
/// This is stored as a minimum and maximum layer.
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct LayerRange {
    /// Minimum layer, inclusive.
    min: Layer,
    /// Maximum layer, inclusive.
    ///
    /// This must be greater than or equal to `min`.
    max: Layer,
}

impl fmt::Display for LayerRange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self { min, max } = self;
        write!(f, "{min}-{max}")
    }
}

impl LayerRange {
    /// Constructs a new inclusive layer range.
    ///
    /// `a` and `b` do not need to be in order.
    pub fn new(a: Layer, b: Layer) -> Self {
        Self {
            min: std::cmp::min(a, b),
            max: std::cmp::max(a, b),
        }
    }

    /// Constructs a layer range containing a single layer.
    pub fn from_layer(layer: Layer) -> Self {
        Self {
            min: layer,
            max: layer,
        }
    }

    /// Returns the minimum layer in the range
    #[doc(alias = "low")]
    pub fn start(self) -> Layer {
        self.min
    }

    /// Returns the maximum layer in the range.
    #[doc(alias = "high")]
    pub fn end(self) -> Layer {
        self.max
    }

    /// Returns the number of layers in the range.
    #[allow(clippy::len_without_is_empty)] // never empty
    pub fn len(self) -> usize {
        self.max.to_usize() - self.min.to_usize() + 1
    }

    /// Clamps the layer range to the range `1..=layer_count`.
    ///
    /// Returns `None` if the resulting layer range would be empty.
    pub fn clamp_to_layer_count(self, layer_count: u16) -> Option<Self> {
        if self.min.to_u16() > layer_count {
            None
        } else {
            Some(Self {
                min: self.min,
                max: self.max.clamp_to_layer_count(layer_count)?,
            })
        }
    }

    /// Returns the single layer in the range if it has only one element, or
    /// `None` othewise.
    pub(crate) fn to_single_layer(self) -> Option<Layer> {
        (self.min == self.max).then_some(self.min)
    }

    /// Returns the union of two ranges, if the result is a contiguous range.
    pub(crate) fn union(self, other: Self) -> Option<Self> {
        let combined = Self {
            min: std::cmp::min(self.min, other.min),
            max: std::cmp::max(self.max, other.max),
        };
        (combined.len() <= self.len() + other.len()).then_some(combined)
    }
}

impl From<Layer> for LayerRange {
    fn from(layer: Layer) -> Self {
        Self::from_layer(layer)
    }
}

impl IntoIterator for LayerRange {
    type Item = Layer;

    type IntoIter = std::iter::Map<std::ops::RangeInclusive<u16>, fn(u16) -> Layer>;

    fn into_iter(self) -> Self::IntoIter {
        (self.min.to_u16()..=self.max.to_u16())
            .map(|i| Layer::new(i).expect("layer range produced out-of-bounds layer"))
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for LayerRange {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(serde::Deserialize)]
        struct LayerRangeSerdeProxy {
            min: Layer,
            max: Layer,
        }

        LayerRangeSerdeProxy::deserialize(deserializer).map(|proxy| Self::new(proxy.min, proxy.max))
    }
}

#[cfg(test)]
impl proptest::arbitrary::Arbitrary for LayerRange {
    type Parameters = ();

    fn arbitrary_with((): Self::Parameters) -> Self::Strategy {
        use proptest::prelude::Strategy;

        (Layer::arbitrary(), Layer::arbitrary())
            .prop_map(|(a, b)| LayerRange::new(a, b))
            .boxed()
    }

    type Strategy = proptest::strategy::BoxedStrategy<Self>;
}
