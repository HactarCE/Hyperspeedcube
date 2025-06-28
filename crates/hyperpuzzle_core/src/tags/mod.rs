//! Tags for classifying puzzles.
//!
//! Puzzles are classified using "tags," which are arranged hierarchically and
//! associated to values (typically boolean). Tags are usually written with a
//! leading `#`, but the `#` is not part of the tag's name.
//!
//! For example, `#shape/3d/cube` is a direct subtag of `#shape/3d`.
//!
//! A tag is **present** for a puzzle if the tag or any of its subtags has a
//! non-null, non-false value associated to it for that puzzle.
//!
//! Tags are arranged in a menu for filtering puzzles.
//!
//! Tags and the tag menu are defined in `tags.kdl` at the crate root.

use std::collections::HashMap;
use std::sync::Arc;

mod menu;
mod set;

pub use menu::AllTags;
pub use set::TagSet;
use strum::EnumString;

lazy_static! {
    /// Catalog of known tags, defined in `tags.kdl`.
    pub static ref TAGS: AllTags = AllTags::load();
}

/// How to display a node in the tag menu.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TagDisplay {
    /// Display child tags inline.
    Inline,
    /// Display a submenu with this label.
    Submenu(Arc<str>),
}

/// Value assigned to a tag.
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
pub enum TagValue {
    /// Not present (allowed for any type)
    #[default]
    False,
    /// Present (allowed only for boolean)
    True,
    /// Present due to subtag
    Inherited,
    /// Integer
    Int(i64),
    /// String
    Str(String),
    /// List of strings
    StrList(Vec<String>),
    /// Puzzle ID
    Puzzle(String),
}
impl From<bool> for TagValue {
    fn from(value: bool) -> Self {
        match value {
            true => TagValue::True,
            false => TagValue::False,
        }
    }
}
impl TagValue {
    /// Returns the value if it is an integer, or `None` otherwise.
    pub fn as_int(&self) -> Option<i64> {
        match self {
            TagValue::Int(i) => Some(*i),
            _ => None,
        }
    }

    /// Returns the value if it is a string, or `None` otherwise.
    pub fn as_str(&self) -> Option<&str> {
        match self {
            TagValue::Str(s) => Some(s),
            _ => None,
        }
    }

    /// Returns the value if it is a string list, or `None` otherwise.
    pub fn as_str_list(&self) -> Option<&[String]> {
        match self {
            TagValue::StrList(vec) => Some(vec),
            _ => None,
        }
    }
    /// Returns a mutable reference to the value if it is a string list, or
    /// `None` otherwise.
    pub fn as_str_list_mut(&mut self) -> Option<&mut Vec<String>> {
        match self {
            TagValue::StrList(vec) => Some(vec),
            _ => None,
        }
    }

    /// Returns the value as a string list if it is a string list, string, or
    /// puzzle ID.
    pub fn to_strings(&self) -> &[String] {
        match self {
            TagValue::Str(s) => std::slice::from_ref(s),
            TagValue::StrList(vec) => vec,
            TagValue::Puzzle(p) => std::slice::from_ref(p),
            _ => &[],
        }
    }

    /// Returns whether the value is anything other than `false`.
    pub fn is_present(&self) -> bool {
        match self {
            TagValue::False => false,
            _ => true,
        }
    }
    /// Returns whether a tag value meets a search for a specific tag value. If
    /// `value_str` is `None`, returns whether the tag is present.
    pub fn meets_search_criterion(&self, value_str: Option<&str>) -> bool {
        if let Some(value_str) = value_str {
            match self {
                TagValue::False => value_str.eq_ignore_ascii_case("false") || value_str == "0",
                TagValue::True | TagValue::Inherited => {
                    value_str.eq_ignore_ascii_case("true") || value_str == "1"
                }
                TagValue::Int(i) => value_str == i.to_string(),
                TagValue::Str(s) => value_str.eq_ignore_ascii_case(s),
                TagValue::StrList(vec) => vec.iter().any(|s2| value_str.eq_ignore_ascii_case(s2)),
                TagValue::Puzzle(s) => value_str.eq_ignore_ascii_case(s),
            }
        } else {
            self.is_present()
        }
    }
}

/// Type of value that a tag is expected to store.
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, Hash, EnumString)]
#[strum(serialize_all = "snake_case")]
pub enum TagType {
    /// Boolean (present vs. not present)
    #[default]
    Bool,
    /// Integer
    Int,
    /// String
    Str,
    /// List of strings
    StrList,
    /// Puzzle ID
    Puzzle,
}

/// Info about a tag.
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
pub struct TagData {
    /// Name of the tag.
    pub name: Arc<str>,
    /// Type of data to store for the tag value.
    pub ty: TagType,
    /// Whether the tag can only be added automatically.
    pub auto: bool,
}

/// Node in the tag menu hierarchy.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TagMenuNode {
    /// Schema version.
    Schema(u64),
    /// Heading.
    Heading(Arc<str>),
    /// Separator.
    Separator,
    /// Tag entry and/or submenu.
    Tag {
        /// Tag name, if there is one.
        name: Option<Arc<str>>,
        /// Type of data for the tag.
        ty: TagType,
        /// Display mode for the tag or category.
        display: TagDisplay,
        /// Subtags inside this category.
        subtags: Vec<TagMenuNode>,

        /// Whether the value for this tag may only be autogenerated.
        auto: bool,
        /// Tag schema version after which this tag is expected to be specified
        /// on all puzzles built into the program.
        expected: Option<u64>,
        /// Whether this tag should be hidden in the tag menu.
        hidden: bool,
        /// Whether this tag should display a list of all possible values in the
        /// menu.
        list: bool,
    },
}
impl TagMenuNode {
    fn name(&self) -> Option<&Arc<str>> {
        match self {
            TagMenuNode::Schema(_) => None,
            TagMenuNode::Heading(_) | TagMenuNode::Separator => None,
            TagMenuNode::Tag { name, .. } => name.as_ref(),
        }
    }
}
