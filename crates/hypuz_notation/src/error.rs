//! Error types.

use std::num::ParseIntError;

/// Error produced when inverting a node list.
#[derive(thiserror::Error, Debug, Clone, PartialEq, Eq, Hash)]
pub enum InvertError {
    /// NISS node cannot be inverted
    #[error("NISS node cannot be inverted")]
    NissNodeCannotBeInverted,
    /// Integer overflow
    ///
    /// This occurs when negating the minimum integer value.
    #[error("integer overflow")]
    IntegerOverflow,
}

/// Error produced when parsing a layer number.
#[derive(thiserror::Error, Debug, Clone, PartialEq, Eq)]
pub enum ParseLayerError {
    /// Integer parse error
    #[error("{0}")]
    ParseInt(#[from] ParseIntError),
    /// Layer number out of range
    #[error("layer number out of range")]
    OutOfRange,
}
