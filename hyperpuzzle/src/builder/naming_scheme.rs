use std::collections::{HashMap, HashSet};
use std::hash::Hash;

use hypermath::IndexNewtype;
use regex::Regex;

use crate::{Axis, Color, PieceType, Twist};

use super::name::TooManyNames;
use super::NameSet;

/// Mutable assignment of names to elements. By default, all elements are
/// unnamed.
#[derive(Debug, Clone)]
pub struct NamingScheme<I> {
    ids_to_names: HashMap<I, NameSet>,
    names_to_ids: HashMap<String, I>,

    ids_to_display_names: HashMap<I, String>,
    display_name_set: HashSet<String>,

    name_validation_regex: &'static Regex,
}
impl<I: Nameable> Default for NamingScheme<I> {
    fn default() -> Self {
        Self::new(I::whole_name_regex())
    }
}
impl<I> NamingScheme<I> {
    /// Constructs a new naming scheme with no elements.
    pub fn new(name_validation_regex: &'static Regex) -> Self {
        Self {
            ids_to_names: Default::default(),
            names_to_ids: Default::default(),

            ids_to_display_names: Default::default(),
            display_name_set: Default::default(),

            name_validation_regex,
        }
    }

    /// Returns the source regex used to validate names.
    pub fn regex(&self) -> &'static Regex {
        &self.name_validation_regex
    }
}
impl<I: Clone + Hash + Eq> NamingScheme<I> {
    /// Returns a map from IDs to names.
    pub fn ids_to_names(&self) -> &HashMap<I, NameSet> {
        &self.ids_to_names
    }
    /// Returns a map from IDs to display names.
    pub fn ids_to_display_names(&self) -> &HashMap<I, String> {
        &self.ids_to_display_names
    }
    /// Returns a map from names to IDs.
    pub fn names_to_ids(&self) -> &HashMap<String, I> {
        &self.names_to_ids
    }

    /// Returns the name of an element, or `None` if the element has not been
    /// assigned a name.
    pub fn get(&self, id: I) -> Option<&NameSet> {
        self.ids_to_names.get(&id)
    }
    /// Returns the display name of an element, or `None` if the element has not
    /// been assigned a display name.
    pub fn get_display(&self, id: I) -> Option<String> {
        self.ids_to_display_names.get(&id).cloned()
    }
    /// Assigns a name to an element, returning an error if there is a name
    /// conflict.
    ///
    /// If the name is invalid, `warn_fn` is called with info about what went
    /// wrong and an empty name is assigned instead of halting execution flow.
    pub fn set_name(&mut self, id: I, name: Option<NameSet>, warn_fn: impl Fn(BadName)) {
        // Remove from `ids_to_names`.
        let old_name = self.ids_to_names.remove(&id);

        // Compute the set of old names.
        let old_strings = match &old_name {
            Some(name_set) => match name_set.string_set() {
                Ok(strings) => strings,
                Err(e) => {
                    warn_fn(BadName::TooManyNames(e));
                    return; // shouldn't ever happen
                }
            },
            None => vec![],
        };

        // Remove from `names_to_ids`.
        for old_string in old_strings {
            self.names_to_ids.remove(&old_string);
        }

        // Compute the set of new names.
        let strings = match &name {
            Some(name_set) => match name_set.string_set() {
                Ok(strings) => {
                    if strings.is_empty() {
                        warn_fn(BadName::EmptySet);
                        return;
                    }
                    strings
                }
                Err(e) => {
                    warn_fn(BadName::TooManyNames(e));
                    return;
                }
            },
            None => vec![],
        };

        // Validate the new names.
        for s in &strings {
            if !self.name_validation_regex.is_match(s) {
                warn_fn(BadName::InvalidName { name: s.clone() });
                return;
            } else if self.names_to_ids.contains_key(s) {
                warn_fn(BadName::AlreadyTaken { name: s.clone() });
                return;
            }
        }

        // Update `ids_to_names`.
        if let Some(new_name) = name {
            self.ids_to_names.insert(id.clone(), new_name);
        };

        // Update `names_to_ids`.
        for s in strings {
            self.names_to_ids.insert(s, id.clone());
        }
    }

    /// Assigns a display name to an element. No validation is performed.
    pub fn set_display(&mut self, id: I, name: Option<String>, warn_fn: impl Fn(BadName)) {
        // Remove the old name.
        if let Some(old_name) = self.ids_to_display_names.remove(&id) {
            self.display_name_set.remove(&old_name);
        }

        if let Some(new_name) = name {
            // Check that the new name is unique; if so, add it.
            if self.display_name_set.insert(new_name.clone()) {
                self.ids_to_display_names.insert(id, new_name);
            } else {
                warn_fn(BadName::AlreadyTaken { name: new_name });
            }
        }
    }

    /// Assings names to unnamed elements using `autonames`.
    pub fn autoname(
        &mut self,
        len: usize,
        autonames: impl IntoIterator<Item = String>,
        warn_fn: impl Copy + Fn(BadName),
    ) where
        I: IndexNewtype,
    {
        let used_names: HashSet<String> = self.names_to_ids.keys().cloned().collect();
        let mut autonames = autonames.into_iter().filter(|s| !used_names.contains(s));
        for i in I::iter(len) {
            if !self.ids_to_names.contains_key(&i) {
                self.set_name(i, autonames.next().map(|s| s.into()), warn_fn);
            }
        }
    }
}

/// Error indicating a bad name.
#[derive(thiserror::Error, Debug, Clone)]
#[allow(missing_docs)]
pub enum BadName {
    #[error("name {name:?} is already taken")]
    AlreadyTaken { name: String },
    #[error("name {name:?} is invalid")]
    InvalidName { name: String },
    #[error("{0}")]
    TooManyNames(TooManyNames),
    #[error("name set is empty")]
    EmptySet,
}

/// Puzzle element that can be named.
pub trait Nameable {
    /// Returns a regex (including `^` and `$`) that matches a string iff it is
    /// a valid name for this kind of puzzle element.
    fn whole_name_regex() -> &'static Regex;
}
macro_rules! impl_nameable {
    ($type:ty, $regex_str:expr $(,)?) => {
        impl Nameable for $type {
            fn whole_name_regex() -> &'static Regex {
                lazy_static! {
                    static ref CACHED_REGEX: Regex =
                        Regex::new(concat!("^", $regex_str, "$")).expect("bad regex");
                }
                &*CACHED_REGEX
            }
        }
    };
}
impl_nameable!(Color, r"[a-zA-Z_][a-zA-Z0-9_]*");
impl_nameable!(Axis, r"[a-zA-Z_][a-zA-Z0-9_]*");
impl_nameable!(
    PieceType,
    r"[a-zA-Z_][a-zA-Z0-9_]*(/[a-zA-Z_][a-zA-Z0-9_]*)*",
);
impl_nameable!(Twist, r"[a-zA-Z_][a-zA-Z0-9_]*'?");
