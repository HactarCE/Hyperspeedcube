//! Error types.

use std::fmt;

use crate::ti::TypedIndex;

/// Error when a [`crate::ti::TypedIndex`] exceeds its maximum value.
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct IndexOverflow {
    /// Name of the indexing type.
    pub type_name: &'static str,
    /// Maximum allowed index for the indexing type.
    pub max_value: usize,
}

impl fmt::Display for IndexOverflow {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "exceeded maximum {} count of {}",
            self.type_name, self.max_value,
        )
    }
}

impl std::error::Error for IndexOverflow {}

impl IndexOverflow {
    /// Constructs a new overflow error for the type `I`.
    pub fn new<I: TypedIndex>() -> Self {
        Self {
            type_name: I::TYPE_NAME,
            max_value: I::MAX_INDEX,
        }
    }
}

/// Error when a [`crate::ti::TypedIndex`] is out of bounds for a collection.
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct IndexOutOfRange {
    /// Name of the indexing type.
    pub type_name: &'static str,
}

impl fmt::Display for IndexOutOfRange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} index out of range", self.type_name)
    }
}

impl std::error::Error for IndexOutOfRange {}

impl IndexOutOfRange {
    /// Constructs a new out-of-bounds error for the type `I`.
    pub fn new<I: TypedIndex>() -> Self {
        Self {
            type_name: I::TYPE_NAME,
        }
    }
}
