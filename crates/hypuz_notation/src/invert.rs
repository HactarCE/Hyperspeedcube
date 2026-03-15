//! Trait for inverting notation elements.

/// Error produced when inverting notation elements.
#[derive(thiserror::Error, Debug, Clone, PartialEq, Eq, Hash)]
pub enum InvertError {
    /// NISS node cannot be inverted
    ///
    /// This occurs when deeply inverting a NISS node.
    #[error("NISS node cannot be inverted")]
    NissNodeCannotBeInverted,
    /// Integer overflow
    ///
    /// This occurs when negating the minimum integer value.
    #[error("integer overflow")]
    IntegerOverflow,
}

/// Notation element that can be inverted.
pub trait Invert: Sized {
    /// Returns the inverse element. When possible, nodes are inverted by
    /// inverting the multiplier and preserving the contents.
    fn inv(self) -> Result<Self, InvertError>;

    /// Returns the inverse element. When possible, nodes are inverted by
    /// preserving the multiplier and inverting the contents.
    ///
    /// The default implementation calls [`Invert::inv()`].
    fn inv_deep(self) -> Result<Self, InvertError> {
        self.inv()
    }
}
