use std::ops::Index;

use hypermath::{GenericVec, IndexNewtype, IndexOutOfRange};

use super::*;

/// Immutable bi-directional map between IDs and name specifications.
#[derive(Debug, Default, Clone)]
pub struct NameSpecBiMap<I> {
    id_to_name: GenericVec<I, NameSpec>,
    name_to_id: NameSpecMap<I>,
}

impl<I: IndexNewtype> NameSpecBiMap<I> {
    /// Constructs a new empty bi-map.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns whether the map is empty.
    pub fn is_empty(&self) -> bool {
        self.id_to_name.is_empty()
    }
    /// Returns the number of IDs in the map.
    pub fn len(&self) -> usize {
        self.id_to_name.len()
    }

    /// Returns an iterator over IDs.
    pub fn iter_keys(&self) -> hypermath::collections::generic_vec::IndexIter<I> {
        self.id_to_name.iter_keys()
    }
    /// Returns an iterator over names.
    pub fn iter_values(&self) -> impl DoubleEndedIterator<Item = &NameSpec> {
        self.id_to_name.iter_values()
    }
    /// Returns an iterator over ID-name pairs.
    pub fn iter(&self) -> impl DoubleEndedIterator<Item = (I, &NameSpec)> {
        self.id_to_name.iter()
    }

    /// Returns the name associated to an ID, or an error if it is out of range.
    pub fn get(&self, id: I) -> Result<&NameSpec, IndexOutOfRange> {
        self.id_to_name.get(id)
    }
    /// Returns the ID associated to a string, if there is one.
    pub fn id_from_name(&self, name: &str) -> Option<I> {
        self.name_to_id.get(name).copied()
    }
    /// Returns the name by which a string is associated to an ID.
    pub fn look_up_name(&self, name: &str) -> Option<&NameSpec> {
        let id = *self.name_to_id.get(name)?;
        Some(&self.id_to_name[id])
    }

    /// Returns whether there is any ID associated to a string.
    pub fn contains_name(&self, name: &str) -> bool {
        self.id_from_name(name).is_some()
    }
}

/// Non-panicking indexing that returns the preferred name, or `"?"` if there is
/// no value.
impl<I: IndexNewtype> Index<I> for NameSpecBiMap<I> {
    type Output = str;

    fn index(&self, index: I) -> &Self::Output {
        match self.get(index) {
            Ok(name) => &name.preferred,
            Err(_) => "?",
        }
    }
}

/// Mutable bi-directional map between IDs and name specifications.
#[derive(Debug, Default, Clone)]
pub struct NameSpecBiMapBuilder<I> {
    id_to_name: GenericVec<I, Option<NameSpec>>,
    name_to_id: NameSpecMap<I>,
}

impl<I: IndexNewtype> NameSpecBiMapBuilder<I> {
    /// Constructs a new empty bi-map.
    pub fn new() -> Self {
        Self::default()
    }

    /// Associates an ID with a name. If the name is invalid, `autonames` is
    /// used instead. If the autoname is also invalid, returns an error.
    pub fn set_with_fallback(
        &mut self,
        id: I,
        name_spec: Option<String>,
        autonames: &mut AutoNames,
        warn_fn: impl FnOnce(BadName),
    ) -> Result<(), BadName> {
        if let Some(s) = name_spec {
            match self.set(id, Some(s)) {
                Ok(()) => return Ok(()),
                Err(e) => warn_fn(e),
            }
        }
        self.set(id, Some(autonames.next_unused(self)))
    }

    /// Associates an ID with a name.
    pub fn set(&mut self, id: I, name_spec: Option<String>) -> Result<(), BadName> {
        let old_name = self.id_to_name.get(id).ok().and_then(Option::as_ref);
        if let Some(old_name) = old_name {
            let _ = self.name_to_id.remove(&old_name.spec);
        }
        let Some(new_name) = name_spec else {
            return Ok(());
        };
        match self.name_to_id.insert(&new_name, &id) {
            Ok(canonical) => {
                self.id_to_name.extend_to_contain(id);
                self.id_to_name[id] = Some(NameSpec {
                    preferred: preferred_name_from_name_spec(&new_name),
                    spec: new_name,
                    canonical,
                });
                Ok(())
            }
            Err(e) => {
                if let Some(old_name) = old_name {
                    // Revert removal.
                    self.name_to_id.insert(&old_name.spec, &id)?;
                }
                Err(e)
            }
        }
    }

    /// Returns the ID of the element with a name based on the preferred name of
    /// `name_spec`.
    ///
    /// This does not guarantee that the whole pattern is unique.
    pub fn get_from_name_spec(&self, name_spec: &str) -> Option<I> {
        self.id_from_string(&preferred_name_from_name_spec(name_spec))
    }

    /// Returns the name associated to an ID, or `None` if it has not been
    /// assigned.
    pub fn get(&self, id: I) -> Option<&NameSpec> {
        self.id_to_name.get(id).ok().and_then(Option::as_ref)
    }
    /// Returns the ID associated to a string, if there is one.
    pub fn id_from_string(&self, string: &str) -> Option<I> {
        self.name_to_id.get(string).copied()
    }
    /// Returns the name by which a string is associated to an ID.
    pub fn look_up_name(&self, name: &str) -> Option<&NameSpec> {
        let id = self.id_from_string(name)?;
        self.id_to_name[id].as_ref()
    }

    /// Assigns automatic names to any elements that do not yet have names.
    ///
    /// Name specifications are selected in order from `autonames`, skipping any
    /// that are already in use. If `autonames` is exhausted,
    /// [`BadName::ExhaustedAutonames`] is returned.
    pub fn autoname(
        &mut self,
        len: usize,
        mut autonames: impl Iterator<Item = String>,
    ) -> Result<(), BadName> {
        for id in I::iter(len) {
            if self.get(id).is_none() {
                let new_name_spec = autonames
                    .find(|autoname| self.id_from_string(autoname).is_none())
                    .ok_or(BadName::ExhaustedAutonames)?;
                self.set(id, Some(new_name_spec))?;
                self.get(id).cloned().ok_or(BadName::InternalError)?;
            }
        }
        Ok(())
    }

    /// Builds the name map, or returns `None` if any name is missing.
    pub fn build(self, len: usize) -> Option<NameSpecBiMap<I>> {
        Some(NameSpecBiMap {
            id_to_name: I::iter(len)
                .map(|id| self.get(id).cloned())
                .collect::<Option<GenericVec<I, NameSpec>>>()?,
            name_to_id: self.name_to_id.clone(),
        })
    }
}

impl<I: IndexNewtype> From<NameSpecBiMap<I>> for NameSpecBiMapBuilder<I> {
    fn from(value: NameSpecBiMap<I>) -> Self {
        NameSpecBiMapBuilder {
            id_to_name: value.id_to_name.map(|_, name| Some(name)),
            name_to_id: value.name_to_id.clone(),
        }
    }
}
