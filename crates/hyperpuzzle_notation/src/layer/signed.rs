use std::fmt;
use std::num::NonZeroI16;
use std::ops::Neg;
use std::str::FromStr;

use super::{Layer, LayerRange};
use crate::errors::ParseLayerError;

/// 1-indexed signed layer number.
///
/// Positive layers are indexed from shallowest (1) to deepest. Negative layers
/// are indexed from deepest (-1) to shallowest (`-layer_count`).
///
/// The minimum and maximum allowed values are `±i16::MAX = ±32767`. Zero is not
/// allowed value.
///
/// Internally, this uses `NonZeroI16` so `Option<Layer>` takes up exactly 2
/// bytes.
///
/// This type implements `From<`[`Layer`]`>` and can be negated for convenience.
/// To convert this to a [`Layer`], use [`SignedLayer::resolve()`]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SignedLayer(pub(super) NonZeroI16);

impl SignedLayer {
    /// Minimum allowed signed layer number.
    pub const MIN: Self = Self(NonZeroI16::new(-i16::MAX).unwrap());
    /// Maximum allowed signed layer number.
    pub const MAX: Self = Self(NonZeroI16::new(i16::MAX).unwrap());

    /// Shallowest layer on any axis (layer number 1).
    pub const SHALLOWEST: Self = Self(NonZeroI16::new(1).unwrap());
    /// Deepest layer on any axis (layer number -1).
    pub const DEEPEST: Self = Self(NonZeroI16::new(i16::MAX).unwrap());

    /// Returns the signed layer number as a nonzero `i16`.
    pub const fn to_nonzero_i16(self) -> NonZeroI16 {
        self.0
    }
    /// Returns the signed layer number as an `i16`.
    pub const fn to_i16(self) -> i16 {
        self.to_nonzero_i16().get()
    }
    /// Returns the signed layer number as an `isize`.
    pub const fn to_isize(self) -> isize {
        self.to_i16() as isize
    }

    /// Returns whether the signed layer number is positive.
    ///
    /// Positive numbers count from the shallowest layer to the deepest.
    pub const fn is_positive(self) -> bool {
        self.0.is_positive()
    }
    /// Returns whether the signed layer number is negative.
    ///
    /// Positive numbers count from the deepest layer to the shallowest.
    pub const fn is_negative(self) -> bool {
        self.0.is_negative()
    }

    /// Resolves a layer, given the number of layers on the axis. See the
    /// [`SignedLayer`] documentation for how this mapping works.
    ///
    /// Returns `None` if the layer is out of range.
    pub fn resolve(self, layer_count: u16) -> Option<Layer> {
        if (self.0.abs().get() as u16) <= layer_count {
            if self.is_positive() {
                Some(Layer(self.0))
            } else {
                Layer::new(layer_count.saturating_sub((-self.0).get() as u16 - 1))
            }
        } else {
            None
        }
    }

    /// Returns an unsigned layer, if this is positive. Returns `None` if it is
    /// negative. If the total number of layers on the axis is known, prefer
    /// [`SignedLayer::resolve()`] instead.
    pub const fn to_unsigned(self) -> Option<Layer> {
        if self.is_positive() {
            Some(Layer(self.0))
        } else {
            None
        }
    }

    /// Resolves a layer, given the number of layers on the axis.
    ///
    /// Returns sentinel values for layers that are out of range.
    fn resolve_unclamped(self, layer_count: u16) -> ResolvedLayer {
        match self.resolve(layer_count) {
            Some(i) => ResolvedLayer::Concrete(i),
            None if self.is_positive() => ResolvedLayer::BeyondDeepest,
            None => ResolvedLayer::BeyondShallowest,
        }
    }

    /// Resolves a layer range, given the number of layers on the axis.
    pub(crate) fn resolve_range(range: [Self; 2], layer_count: u16) -> Option<LayerRange> {
        let range @ [a, b] = range.map(|l| l.resolve_unclamped(layer_count));
        if a == b && !matches!(a, ResolvedLayer::Concrete(_)) {
            None
        } else {
            let [a, b] = range.map(|l| match l {
                ResolvedLayer::BeyondShallowest => Layer::SHALLOWEST,
                ResolvedLayer::Concrete(layer) => layer,
                ResolvedLayer::BeyondDeepest => Layer::new_clamped(layer_count),
            });
            Some(LayerRange::new(a, b))
        }
    }

    /// Constructs a new signed layer.
    ///
    /// Returns `None` if `i` is not in the ranges `-32767..=-1` or `1..=32767`.
    pub const fn new(layer: i16) -> Option<Self> {
        if let Some(inner) = NonZeroI16::new(layer)
            && layer != i16::MIN
        {
            Some(Self(inner))
        } else {
            None
        }
    }
}

impl Default for SignedLayer {
    fn default() -> Self {
        Self::SHALLOWEST
    }
}

impl fmt::Display for SignedLayer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for SignedLayer {
    type Err = ParseLayerError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s.parse()?).ok_or(ParseLayerError::OutOfRange)
    }
}

impl From<Layer> for SignedLayer {
    fn from(value: Layer) -> Self {
        value.to_signed()
    }
}

impl Neg for SignedLayer {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Self(-self.0)
    }
}

#[cfg(test)]
impl proptest::arbitrary::Arbitrary for SignedLayer {
    type Parameters = ();

    fn arbitrary_with((): Self::Parameters) -> Self::Strategy {
        use proptest::prelude::Strategy;

        (SignedLayer::MIN.to_i16()..=SignedLayer::MAX.to_i16())
            .prop_filter_map("must be a valid layer", SignedLayer::new)
            .boxed()
    }

    type Strategy = proptest::strategy::BoxedStrategy<Self>;
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub(crate) enum ResolvedLayer {
    BeyondShallowest,
    Concrete(Layer),
    BeyondDeepest,
}
