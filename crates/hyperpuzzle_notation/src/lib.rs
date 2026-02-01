//! Hyper Puzzle Notation parser and serializer.

pub mod charsets;
pub mod common;
pub mod family;
mod parse;
pub mod spanned;
pub mod transform;
pub mod unspanned;

pub use parse::ParseError;
pub use unspanned::*;

/// String type.
pub type Str = lean_string::LeanString;

/// Span in a string of puzzle notation.
pub type Span = chumsky::span::SimpleSpan;
/// Wrapper around a type that includes span information.
pub type Spanned<T> = chumsky::span::SimpleSpanned<T>;

/// Set of features to enable when parsing puzzle notation.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct Features {
    /// Layer mask features.
    pub layers: LayerMaskFeatures,

    /// Whether to allow rotations using generalized rotation (`@`) syntax.
    pub generalized_rotations: bool,

    /// Whether to allow Megaminx scrambling notation.
    ///
    /// Example: `R++ D--`
    pub megaminx: bool,

    /// Whether to allow Square-1 notation.
    ///
    /// Example: `(1,0)/(3,3)/(-1,0)`
    pub sq1: bool,
}

impl Default for Features {
    fn default() -> Self {
        Self::MAXIMAL
    }
}

impl Features {
    /// Minimal feature set with no hypercubing-specific notation.
    pub const MINIMAL: Self = Self {
        layers: LayerMaskFeatures::SIMPLE,
        generalized_rotations: false,
        megaminx: false,
        sq1: false,
    };

    /// Typical 3D puzzle notation, including special notation for specific WCA
    /// puzzles but not hypercubing-specific notation.
    pub const WCA: Self = Self {
        layers: LayerMaskFeatures::SIMPLE,
        generalized_rotations: false,
        megaminx: true,
        sq1: true,
    };

    /// Maximumal feature set, including hypercubing notation and special
    /// notation for specific WCA puzzles.
    pub const MAXIMAL: Self = Self {
        layers: LayerMaskFeatures::HYPERCUBING,
        generalized_rotations: true,
        megaminx: true,
        sq1: true,
    };
}

/// Set of features to enable when parsing layer masks.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct LayerMaskFeatures {
    /// Whether to allow inverting layer masks.
    pub inverting: bool,

    /// Whether to allow layer sets.
    ///
    /// Example: `{1..3, 5}R`
    pub layer_sets: bool,

    /// Whether to allow HSC1-style layer ranges in layer sets.
    ///
    /// Example: `{1-3, 5}R`
    ///
    /// This has no effect when `layer_sets` is `false`.
    pub hsc1_layer_ranges: bool,
}

impl Default for LayerMaskFeatures {
    fn default() -> Self {
        Self::SIMPLE
    }
}

impl LayerMaskFeatures {
    /// Minimal feature set with no hypercubing-specific notation.
    pub const SIMPLE: Self = Self {
        inverting: false,
        layer_sets: false,
        hsc1_layer_ranges: false,
    };

    /// Maximumal feature set, including hypercubing notation.
    pub const HYPERCUBING: Self = Self {
        inverting: true,
        layer_sets: true,
        hsc1_layer_ranges: true,
    };
}

/// Resolves a signed layer to an unsigned layer, clamping it to within the
/// layer range. Returns `None` if `signed_layer` is 0.
pub fn resolve_signed_layer(layer_count: u16, signed_layer: i16) -> Option<u16> {
    Some(resolve_signed_layer_unchecked(layer_count, signed_layer))
        .filter(|l| (1..=layer_count).contains(l))
}

/// Resolves a signed layer range to an unsigned layer, clamping it to within
/// the layer range. Returns `None` if the whole range is out of bounds.
pub fn resolve_signed_layer_range(layer_count: u16, range: [i16; 2]) -> Option<[u16; 2]> {
    let mut range = range.map(|l| resolve_signed_layer_unchecked(layer_count, l));
    range.sort();
    let [lo, hi] = range;
    (hi >= 1 && lo <= layer_count).then_some([lo.max(1), hi.min(layer_count)])
}

fn resolve_signed_layer_unchecked(layer_count: u16, signed_layer: i16) -> u16 {
    if signed_layer < 0 {
        layer_count.saturating_sub((-(signed_layer + 1)) as u16)
    } else {
        signed_layer as u16
    }
}

#[cfg(test)]
mod tests;
