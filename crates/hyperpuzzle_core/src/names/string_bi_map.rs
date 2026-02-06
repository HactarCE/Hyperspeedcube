use std::collections::HashMap;
use std::ops::Index;

use hyperpuzzle_util::ti::{IndexOutOfRange, TiVec, TypedIndex, TypedIndexIter};

use super::BadName;

/// Immutable bi-directional map between IDs and strings.
#[derive(Debug, Default, Clone)]
pub struct StringBiMap<I> {
    id_to_string: TiVec<I, String>,
    string_to_id: HashMap<String, I>,
}

impl<I: TypedIndex> StringBiMap<I> {
    /// Constructs a new empty bi-map.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns whether the map is empty.
    pub fn is_empty(&self) -> bool {
        self.id_to_string.is_empty()
    }
    /// Returns the number of IDs in the map.
    pub fn len(&self) -> usize {
        self.id_to_string.len()
    }

    /// Returns an iterator over IDs.
    pub fn iter_keys(&self) -> TypedIndexIter<I> {
        self.id_to_string.iter_keys()
    }
    /// Returns an iterator over strings.
    pub fn iter_values(&self) -> impl DoubleEndedIterator<Item = &String> {
        self.id_to_string.iter_values()
    }
    /// Returns an iterator over ID-string pairs.
    pub fn iter(&self) -> impl DoubleEndedIterator<Item = (I, &String)> {
        self.id_to_string.iter()
    }

    /// Returns the string associated to an ID, or an error if it is out of
    /// range.
    pub fn get(&self, id: I) -> Result<&String, IndexOutOfRange> {
        self.id_to_string.get(id)
    }
    /// Returns the ID associated to a string, if there is one.
    pub fn id_from_string(&self, string: &str) -> Option<I> {
        self.string_to_id.get(string).copied()
    }

    /// Returns whether there is any ID associated to a string.
    pub fn contains_string(&self, string: &str) -> bool {
        self.id_from_string(string).is_some()
    }
}

/// Non-panicking indexing that returns `"?"` if there is no value.
impl<I: TypedIndex> Index<I> for StringBiMap<I> {
    type Output = str;

    fn index(&self, index: I) -> &Self::Output {
        self.get(index).map(String::as_str).unwrap_or("?")
    }
}

/// Immutable bi-directional map between IDs and strings.
#[derive(Debug, Default, Clone)]
pub struct StringBiMapBuilder<I> {
    id_to_string: TiVec<I, Option<String>>,
    string_to_id: HashMap<String, I>,
}

impl<I: TypedIndex> StringBiMapBuilder<I> {
    /// Constructs a new empty bi-map.
    pub fn new() -> Self {
        Self::default()
    }

    /// Associates an ID with a string.
    pub fn set(&mut self, id: I, string: Option<String>) -> Result<(), BadName> {
        if let Some(s) = &string {
            if s.is_empty() {
                return Err(BadName::Empty);
            }
            // TODO: consider more validation
            if let Some(&other_id) = self.string_to_id.get(s)
                && other_id != id
            {
                return Err(BadName::AlreadyTaken { name: s.clone() });
            }
            self.string_to_id.insert(s.clone(), id);
        } else if let Some(old_string) = self.id_to_string.get_opt(id) {
            self.string_to_id.remove(old_string);
        }
        self.id_to_string.extend_to_contain(id);
        self.id_to_string[id] = string;
        Ok(())
    }

    /// Returns the string associated to an ID, or `None` if it has not been
    /// assigned.
    pub fn get(&self, id: I) -> Option<&String> {
        self.id_to_string.get(id).ok().and_then(Option::as_ref)
    }
    /// Returns the ID associated to a string, if there is one.
    pub fn id_from_string(&self, string: &str) -> Option<I> {
        self.string_to_id.get(string).copied()
    }

    /// Assigns automatic names to any elements that do not yet have names.
    ///
    /// Names are selected in order from `autonames`, skipping any that are
    /// already in use. If `autonames` is exhausted,
    /// [`BadName::ExhaustedAutonames`] is returned.
    pub fn autoname(
        &mut self,
        len: usize,
        mut autonames: impl Iterator<Item = String>,
    ) -> Result<(), BadName> {
        for id in I::iter(len) {
            if self.get(id).is_none() {
                let new_name = autonames
                    .find(|autoname| self.id_from_string(autoname).is_none())
                    .ok_or(BadName::ExhaustedAutonames)?;
                self.set(id, Some(new_name))?;
                self.get(id).cloned().ok_or(BadName::InternalError)?;
            }
        }
        Ok(())
    }

    /// Builds the string map, or returns `None` if any string is missing.
    pub fn build(self, len: usize) -> Option<StringBiMap<I>> {
        Some(StringBiMap {
            id_to_string: I::iter(len)
                .map(|id| self.get(id).cloned())
                .collect::<Option<TiVec<I, String>>>()?,
            string_to_id: self.string_to_id.clone(),
        })
    }
}

impl<I: TypedIndex> From<StringBiMap<I>> for StringBiMapBuilder<I> {
    fn from(value: StringBiMap<I>) -> Self {
        StringBiMapBuilder {
            id_to_string: value.id_to_string.map(|_, string| Some(string)),
            string_to_id: value.string_to_id.clone(),
        }
    }
}
