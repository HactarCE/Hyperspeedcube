use std::{collections::HashMap, fmt, ops::Index, sync::Arc};

use hypuz_notation::Str;
use hypuz_util::ti::{IndexOutOfRange, TiVec, TypedIndex};

/// Bidirectional map between an index type `I` and string names.
///
/// Multiple names may map to the same index, but each index only maps to one
/// name (its "canonical" name). If an index maps to a name, then that name must
/// map back to that index. This implies that every index has a unique canonical
/// name.
///
/// Names must be nonempty.
#[derive(Clone)]
pub struct Names<I> {
    list: TiVec<I, Str>,
    name_to_id: Arc<dyn Send + Sync + Fn(&str) -> Option<I>>,
}

impl<I: TypedIndex> fmt::Debug for Names<I> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Names")
            .field("names", &self.list)
            .finish_non_exhaustive()
    }
}

impl<I: TypedIndex> Index<I> for Names<I> {
    type Output = Str;

    fn index(&self, index: I) -> &Self::Output {
        &self.list[index]
    }
}

impl<I: TypedIndex> Default for Names<I> {
    fn default() -> Self {
        Self::new_empty()
    }
}

impl<I: TypedIndex> Names<I> {
    /// Constructs an empty name map.
    pub fn new_empty() -> Self {
        Self {
            list: TiVec::new(),
            name_to_id: Arc::new(|_| None),
        }
    }

    /// Constructs a name map that contains only the canonical names.
    ///
    /// Returns an error if any name is empty or duplicated.
    pub fn new_simple(names: TiVec<I, Str>) -> Result<Self, NameError> {
        let map_back: HashMap<Str, I> = names.iter().map(|(i, s)| (s.clone(), i)).collect();

        for (id, name) in &names {
            if name.is_empty() {
                return Err(NameError::Empty {
                    type_name: I::TYPE_NAME,
                    id: id.to_index(),
                });
            }
            if let Some(&id2) = map_back.get(name)
                && id2 != id
            {
                return Err(NameError::Conflict {
                    type_name: I::TYPE_NAME,
                    name: name.clone(),
                    id1: id.to_index(),
                    id2: id2.to_index(),
                });
            }
        }

        Ok(Self {
            list: names,
            name_to_id: Arc::new(move |s| map_back.get(s).copied()),
        })
    }

    /// Constructs a name map.
    ///
    /// Returns an error if any name is empty.
    ///
    /// `name_to_id(names[i]) == Some(i)` must hold for every index. If it does
    /// not, an error is returned.
    pub fn new(
        names: TiVec<I, Str>,
        name_to_id: impl 'static + Send + Sync + Fn(&str) -> Option<I>,
    ) -> Result<Self, NameError> {
        for (id, name) in &names {
            if name.is_empty() {
                return Err(NameError::Empty {
                    type_name: I::TYPE_NAME,
                    id: id.to_index(),
                });
            }
            let got_id = name_to_id(name);
            if got_id != Some(id) {
                return Err(NameError::DoesntMapBack {
                    type_name: I::TYPE_NAME,
                    name: name.clone(),
                    expected_id: id.to_index(),
                    got_id: got_id.map(|i| i.to_index()),
                });
            }
        }
        Ok(Self {
            list: names,
            name_to_id: Arc::new(name_to_id),
        })
    }

    /// Returns the list of canonical names.
    pub fn list(&self) -> &TiVec<I, Str> {
        &self.list
    }

    /// Returns the number of elements.
    pub fn len(&self) -> usize {
        self.list.len()
    }

    /// Returns whether there are no elements.
    pub fn is_empty(&self) -> bool {
        self.list.is_empty()
    }

    /// Returns the name for an element.
    pub fn get(&self, index: I) -> Result<&Str, IndexOutOfRange> {
        self.list.get(index)
    }

    /// Returns an element from its name.
    ///
    /// This should be fast, but in general it may run arbitrary code and the
    /// results are not cached.
    pub fn lookup(&self, name: &str) -> Option<I> {
        if name.is_empty() {
            return None;
        }
        (self.name_to_id)(name).inspect(|&i| {
            #[cfg(debug_assertions)]
            let _ = &self.list[i]; // panic if index is out of bounds
        })
    }

    /// Returns whether the given name maps to an element.
    ///
    /// This is equivalent to `self.lookup(name).is_some()`.
    pub fn contains_name(&self, name: &str) -> bool {
        self.lookup(name).is_some()
    }
}

#[derive(thiserror::Error, Debug, Clone, PartialEq, Eq)]
pub enum NameError {
    #[error("name for {type_name} #{id} cannot be empty")]
    Empty { type_name: &'static str, id: usize },
    #[error("{type_name} name conflict: #{id1} and #{id2} are both {name:?}")]
    Conflict {
        type_name: &'static str,
        name: Str,
        id1: usize,
        id2: usize,
    },
    #[error("{type_name} #{expected_id} has name {name:?}, but {name:?} maps to {got_id:?}")]
    DoesntMapBack {
        type_name: &'static str,
        name: Str,
        expected_id: usize,
        got_id: Option<usize>,
    },
}
