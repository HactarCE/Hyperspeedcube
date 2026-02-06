//! Hyper Puzzle Notation parser and serializer.

pub mod bracketed;
pub mod charsets;
pub mod common;
pub mod error;
pub mod family;
pub mod layer;
mod parse;
pub mod spanned;
pub mod unspanned;

pub use error::InvertError;
pub use layer::*;
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
    /// Layer prefix features.
    pub layers: LayerFeatures,

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
        layers: LayerFeatures::SIMPLE,
        generalized_rotations: false,
        megaminx: false,
        sq1: false,
    };

    /// Typical 3D puzzle notation, including special notation for specific WCA
    /// puzzles but not hypercubing-specific notation.
    pub const WCA: Self = Self {
        layers: LayerFeatures::SIMPLE,
        generalized_rotations: false,
        megaminx: true,
        sq1: true,
    };

    /// Maximumal feature set, including hypercubing notation and special
    /// notation for specific WCA puzzles.
    pub const MAXIMAL: Self = Self {
        layers: LayerFeatures::HYPERCUBING,
        generalized_rotations: true,
        megaminx: true,
        sq1: true,
    };
}

/// Set of features to enable when parsing layer prefixes.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct LayerFeatures {
    /// Whether to allow inverting layer prefixes.
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

impl Default for LayerFeatures {
    fn default() -> Self {
        Self::SIMPLE
    }
}

impl LayerFeatures {
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

#[cfg(test)]
mod tests;
