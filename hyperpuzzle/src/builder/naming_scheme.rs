use std::collections::{HashMap, HashSet};
use std::hash::Hash;

use hypermath::IndexNewtype;
use regex::Regex;

lazy_static! {
    static ref NAME_REGEX_FULL_MATCH: Regex =
        Regex::new(&format!(r"^{}$", crate::NAME_REGEX)).expect("bad compile-time regex");
}

/// Mutable assignment of names to elements. By default, all elements are
/// unnamed.
#[derive(Debug, Clone)]
pub struct NamingScheme<I> {
    ids_to_names: HashMap<I, String>,
    ids_to_display_names: HashMap<I, String>,
    names_to_ids: HashMap<String, I>,
}
impl<I> Default for NamingScheme<I> {
    fn default() -> Self {
        Self {
            ids_to_names: Default::default(),
            ids_to_display_names: Default::default(),
            names_to_ids: Default::default(),
        }
    }
}
impl<I: Clone + Hash + Eq> NamingScheme<I> {
    /// Constructs a new naming scheme with no elements.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns a map from IDs to names.
    pub fn ids_to_names(&self) -> &HashMap<I, String> {
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
    pub fn get(&self, id: I) -> Option<String> {
        self.ids_to_names.get(&id).cloned()
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
    pub fn set_name(&mut self, id: I, mut name: Option<String>, warn_fn: impl Fn(BadName)) {
        let old_name = self.ids_to_names.get(&id);

        // Canonicalize the name by removing leading & trailing whitespace.
        if let Some(new_name) = &mut name {
            *new_name = new_name.trim().to_string();
            if new_name.is_empty() {
                name = None;
            }
        }

        // If the new name is the same, do nothing.
        if name.as_ref() == old_name {
            return;
        }

        // Remove the old name.
        if let Some(old_name) = old_name {
            self.names_to_ids.remove(old_name);
        }

        // Validate the new name.
        if let Some(new_name) = name.clone() {
            // Reserved symbols for autogenerated names
            if !NAME_REGEX_FULL_MATCH.is_match(&new_name) {
                warn_fn(BadName::InvalidName { name: new_name });
                name = None;
            } else if self.names_to_ids.contains_key(&new_name) {
                warn_fn(BadName::AlreadyTaken { name: new_name });
                name = None;
            }
        }

        // Update `names_to_ids`.
        if let Some(new_name) = name.clone() {
            self.names_to_ids.insert(new_name, id.clone());
        }

        // Update `ids_to_names`.
        match name {
            Some(new_name) => self.ids_to_names.insert(id, new_name),
            None => self.ids_to_names.remove(&id),
        };
    }

    /// Assigns a display name to an element. No validation is performed.
    pub fn set_display(&mut self, id: I, name: Option<String>) {
        match name {
            Some(s) => self.ids_to_display_names.insert(id, s.trim().to_string()),
            None => self.ids_to_display_names.remove(&id),
        };
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
                self.set_name(i, autonames.next(), warn_fn);
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
}
