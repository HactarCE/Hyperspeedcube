use thiserror::Error;

/// Error produced when inverting a node list.
#[derive(Error, Debug, Clone, PartialEq, Eq, Hash)]
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
