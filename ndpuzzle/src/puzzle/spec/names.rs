use ahash::AHashMap;
use anyhow::{ensure, Result};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;

/// Specification for a set of names.
#[derive(Serialize, Deserialize, Debug, Default, Clone)]
#[serde(deny_unknown_fields)]
pub struct NameSetSpec {
    /// Optional prefix before each name.
    pub prefix: Option<String>,
    /// Name to give each member.
    pub names: Option<Vec<String>>,
}

/// Helper struct to give names to things.
#[derive(Debug)]
pub(super) struct Namer<P, T> {
    pub prefix_iter: P,
    pub by_name: AHashMap<String, T>,
    pub type_of_thing: &'static str,
}
impl<P: Iterator<Item = String>, T: Copy> Namer<P, T> {
    /// Returns the names for a set of things.
    pub fn assign_names<'a>(
        &'a mut self,
        name_set: &'a NameSetSpec,
        elements: impl 'a + IntoIterator<Item = T>,
    ) -> impl 'a + Iterator<Item = Result<String>> {
        let prefix = if let Some(prefix) = &name_set.prefix {
            prefix.clone()
        } else if name_set.names.is_some() {
            "".to_string()
        } else {
            self.prefix_iter.next().unwrap()
        };

        let user_specified_names = name_set.names.iter().flatten().map(Cow::Borrowed);
        let unprefixed_names =
            user_specified_names.chain(crate::util::letters_lower().map(Cow::Owned));

        unprefixed_names
            .map(move |s| format!("{prefix}{s}"))
            .zip(elements)
            .map(|(name, thing)| {
                // Ensure the name is unique.
                let is_name_unique = self.by_name.insert(name.clone(), thing).is_none();
                ensure!(
                    is_name_unique,
                    "{} names must be unique; multiple have name {name:?}",
                    self.type_of_thing,
                );
                Ok(name)
            })
    }
}
