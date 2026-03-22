//! Error produced when expanding notation elements.

/// Error produced when expanding notation elements.
#[derive(thiserror::Error, Debug, Clone, PartialEq, Eq, Hash)]
pub enum ExpandError {
    /// Error produced when inverting a node.
    #[error("{0}")]
    Invert(#[from] crate::invert::InvertError),
    /// NISS node cannot be expanded
    ///
    /// This occurs when expanding a NISS node.
    #[error("NISS node cannot be expanded")]
    NissNodeCannotBeExpanded,
}
