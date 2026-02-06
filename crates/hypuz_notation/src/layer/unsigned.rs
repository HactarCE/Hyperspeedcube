use std::fmt;
use std::num::{NonZeroI16, NonZeroU16};
use std::str::FromStr;

use super::SignedLayer;
use crate::error::ParseLayerError;

/// 1-indexed unsigned layer number.
///
/// Layers are indexed from shallowest (1) to deepest (`layer_count`).
///
/// The maximum allowed value is `i16::MAX = 32767`. Zero is not allowed value.
///
/// Internally, this uses `NonZeroI16` so `Option<Layer>` takes up exactly 2
/// bytes.
///
/// This type implements `Into<`[`SignedLayer`]`>` for convenience.
///
/// ## Typed index
///
/// When the `typed_index` feature is envaled, `Layer` implements
/// [`hypuz_util::ti::TypedIndex`] with an offset of 1: `Layer(1)`
/// corresponds to index 0, `Layer(2)` corresponds to index 1, etc.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Layer(pub(super) NonZeroI16); // invariant: always positive

impl Layer {
    /// Minimum allowed layer number.
    pub const MIN: Self = Self(NonZeroI16::new(1).unwrap());
    /// Maximum allowed layer number.
    pub const MAX: Self = Self(NonZeroI16::new(i16::MAX).unwrap());

    /// Shallowest layer on any axis (layer number 1).
    pub const SHALLOWEST: Self = Self(NonZeroI16::new(1).unwrap());

    /// Returns the layer number as a nonzero `i16`.
    pub const fn to_nonzero_i16(self) -> NonZeroI16 {
        self.0
    }
    /// Returns the layer number as a nonzero `u16`.
    pub const fn to_nonzero_u16(self) -> NonZeroU16 {
        self.0.cast_unsigned()
    }
    /// Returns the layer number as an `i16`.
    pub const fn to_i16(self) -> i16 {
        self.to_nonzero_i16().get()
    }
    /// Returns the layer number as a `u16`.
    pub const fn to_u16(self) -> u16 {
        self.to_nonzero_u16().get()
    }
    /// Returns the layer number as an `isize`.
    pub const fn to_isize(self) -> isize {
        self.to_i16() as isize
    }
    /// Returns the layer number as a `usize`.
    pub const fn to_usize(self) -> usize {
        self.to_u16() as usize
    }

    /// Returns the zero-based index corresponding to the layer.
    pub const fn index(self) -> usize {
        self.0.get() as usize - 1
    }

    /// Constructs a new layer.
    ///
    /// Returns `None` if `i` is not in the range `1..=32767`.
    pub const fn new(i: u16) -> Option<Self> {
        if i <= i16::MAX as u16
            && let Some(inner) = NonZeroI16::new(i as i16)
        {
            Some(Self(inner))
        } else {
            None
        }
    }

    /// Constructs a new layer, clamping `i` to the range `1..=32767`.
    pub fn new_clamped(i: u16) -> Self {
        let i16 = i16::try_from(i).unwrap_or(i16::MAX);
        let non_zero_i16 = NonZeroI16::new(i16).unwrap_or(NonZeroI16::MIN);
        Self(non_zero_i16)
    }

    /// Clamps the layer to the range `1..=layer_count`.
    ///
    /// Returns `None` if `layer_count` is zero.
    #[must_use]
    pub const fn clamp_to_layer_count(self, layer_count: u16) -> Option<Self> {
        if self.to_u16() > layer_count {
            Layer::new(layer_count)
        } else {
            Some(self)
        }
    }

    /// Converts the layer to a positive [`SignedLayer`].
    pub const fn to_signed(self) -> SignedLayer {
        SignedLayer(self.0)
    }
}

impl Default for Layer {
    fn default() -> Self {
        Self::SHALLOWEST
    }
}

impl fmt::Display for Layer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for Layer {
    type Err = ParseLayerError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s.parse()?).ok_or(ParseLayerError::OutOfRange)
    }
}

impl TryFrom<SignedLayer> for Layer {
    type Error = std::num::TryFromIntError;

    fn try_from(value: SignedLayer) -> Result<Self, Self::Error> {
        NonZeroU16::try_from(value.0)?;
        Ok(Self(value.0))
    }
}

#[cfg(feature = "typed_index")]
impl hypuz_util::ti::Fits64 for Layer {
    unsafe fn from_u64(x: u64) -> Self {
        Layer(unsafe { NonZeroI16::new_unchecked(x as i16) })
    }

    fn to_u64(self) -> u64 {
        self.to_u16() as u64
    }
}

#[cfg(feature = "typed_index")]
impl hypuz_util::ti::TypedIndex for Layer {
    const MAX: Self = Layer::MAX;
    const MAX_INDEX: usize = Layer::MAX.to_usize() - 1;
    const TYPE_NAME: &'static str = "Layer";

    fn to_index(self) -> usize {
        self.to_usize() - 1
    }

    fn try_from_index(index: usize) -> Result<Self, hypuz_util::error::IndexOverflow> {
        Self::new((index + 1) as u16).ok_or(hypuz_util::error::IndexOverflow::new::<Self>())
    }
}

#[cfg(test)]
impl proptest::arbitrary::Arbitrary for Layer {
    type Parameters = ();

    fn arbitrary_with((): Self::Parameters) -> Self::Strategy {
        use proptest::prelude::Strategy;

        (Layer::MIN.to_u16()..=Layer::MAX.to_u16())
            .prop_filter_map("must be a valid layer", Layer::new)
            .boxed()
    }

    type Strategy = proptest::strategy::BoxedStrategy<Self>;
}
