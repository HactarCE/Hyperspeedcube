use std::collections::{HashMap, HashSet};
use std::hash::Hash;

use hypermath::IndexNewtype;

/// Mutable assignment of names to elements. By default, all elements are
/// unnamed.
#[derive(Debug, Clone)]
pub struct NamingScheme<I> {
    ids_to_names: HashMap<I, String>,
    names_to_ids: HashMap<String, I>,
}
impl<I> Default for NamingScheme<I> {
    fn default() -> Self {
        Self {
            ids_to_names: Default::default(),
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
    /// Returns a map from names to IDs.
    pub fn names_to_ids(&self) -> &HashMap<String, I> {
        &self.names_to_ids
    }

    /// Returns the name of an element, or `None` if the element has not been
    /// assigned a name.
    pub fn get(&self, id: I) -> Option<String> {
        self.ids_to_names.get(&id).cloned()
    }
    /// Assigns a name to an element, returning an error if there is a name
    /// conflict.
    pub fn set(&mut self, id: I, mut name: Option<String>) -> Result<(), BadName> {
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
            return Ok(());
        }

        // Remove the old name.
        if let Some(old_name) = old_name {
            self.names_to_ids.remove(old_name);
        }

        if let Some(new_name) = name.clone() {
            // Ensure the new name is valid.
            if new_name.starts_with(|c: char| c.is_numeric()) {
                return Err(BadName::InvalidName { name: new_name });
            }

            // Ensure the new name is free.
            if self.names_to_ids.contains_key(&new_name) {
                return Err(BadName::AlreadyTaken { name: new_name });
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

        Ok(())
    }

    /// Names unnamed elements using `autonames`.
    pub fn autoname(
        &mut self,
        len: usize,
        autonames: impl IntoIterator<Item = String>,
    ) -> Result<(), BadName>
    where
        I: IndexNewtype,
    {
        let used_names: HashSet<String> = self.names_to_ids.keys().cloned().collect();
        let mut autonames = autonames.into_iter().filter(|s| !used_names.contains(s));
        for i in I::iter(len) {
            if !self.ids_to_names.contains_key(&i) {
                self.set(i, autonames.next())?;
            }
        }
        Ok(())
    }
}

/// Error indicating a bad name
#[derive(thiserror::Error, Debug, Clone)]
#[allow(missing_docs)]
pub enum BadName {
    /// The name is already taken.
    #[error("name {name:?} is already taken")]
    AlreadyTaken { name: String },
    /// The name is invalid.
    #[error("name {name:?} is invalid")]
    InvalidName { name: String },
}
