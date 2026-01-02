//! Types used by the `puzzle` subcommand.

use serde::{Deserialize, Serialize};

use std::collections::HashMap;

/// Common metadata about a puzzle or puzzle generator.
///
/// This is a particularly useful abstraction for displaying the puzzle list.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PuzzleListMetadata {
    /// Internal ID.
    pub id: String,
    /// Semantic version: `[major, minor, patch]`.
    pub version: [u32; 3],
    /// Human-friendly name.
    pub name: String,
    /// Human-friendly aliases.
    pub aliases: Vec<String>,
    /// Set of tags and associated values.
    ///
    /// Inherited tags (such as `shape/4d` from `shape/4d/hypercube`) are not
    /// included.
    pub tags: HashMap<String, TagValue>,
}

/// Value for a tag.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
#[serde(untagged)]
pub enum TagValue {
    /// Boolean
    Bool(bool),
    /// Integer
    Int(i64),
    /// String or puzzle ID
    Str(String),
    /// List of strings
    StrList(Vec<String>),
}

impl From<bool> for TagValue {
    fn from(value: bool) -> Self {
        Self::Bool(value)
    }
}

impl From<i64> for TagValue {
    fn from(value: i64) -> Self {
        Self::Int(value)
    }
}

impl From<String> for TagValue {
    fn from(value: String) -> Self {
        Self::Str(value)
    }
}

impl From<Vec<String>> for TagValue {
    fn from(value: Vec<String>) -> Self {
        Self::StrList(value)
    }
}
