use std::collections::HashMap;
use std::fmt;
use std::hash::Hash;

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
    /// Assigns a name for to element, returning an error if there is a name
    /// conflict.
    pub fn set(&mut self, id: I, name: Option<String>) -> Result<(), NameConflict> {
        let old_name = self.ids_to_names.get(&id);

        // If the new name is the same, do nothing.
        if name.as_ref() == old_name {
            return Ok(());
        }

        // Remove the old name.
        if let Some(old_name) = old_name {
            self.names_to_ids.remove(old_name);
        }

        // Ensure the new name is free.
        if let Some(new_name) = name.clone() {
            if self.names_to_ids.contains_key(&new_name) {
                return Err(NameConflict { name: new_name });
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
}

#[derive(Debug, Clone)]
pub struct NameConflict {
    pub name: String,
}
impl fmt::Display for NameConflict {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "name {:?} already taken", self.name)
    }
}
impl std::error::Error for NameConflict {}
