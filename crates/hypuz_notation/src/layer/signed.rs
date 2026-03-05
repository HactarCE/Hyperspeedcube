use std::fmt;
use std::num::NonZeroI16;
use std::ops::Neg;
use std::str::FromStr;

use super::Layer;
use crate::error::ParseLayerError;

/// 1-indexed signed layer number.
///
/// Positive layers are indexed from shallowest (1) to deepest. On axes that
/// support negative layer numbers, negative layers are indexed from deepest
/// (-1) to shallowest (`-layer_count`).
///
/// The minimum and maximum allowed values are `±i16::MAX = ±32767`. Zero is not
/// an allowed value.
///
/// Internally, this uses `NonZeroI16` so `Option<Layer>` takes up exactly 2
/// bytes.
///
/// This type implements `From<`[`Layer`]`>` and can be negated for convenience.
/// To convert this to a [`Layer`], use [`crate::AxisLayersInfo::resolve()`]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
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

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for SignedLayer {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        i16::deserialize(deserializer).and_then(|i| {
            Self::new(i).ok_or_else(|| serde::de::Error::custom("invalid signed layer"))
        })
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
