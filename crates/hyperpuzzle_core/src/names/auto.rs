use std::fmt;

use hyperpuzzle_util::ti::TypedIndex;

use super::NameSpecBiMapBuilder;

/// Automatic names for puzzle elements.
pub struct AutoNames(Box<dyn Send + Sync + Iterator<Item = String>>);

impl Iterator for AutoNames {
    type Item = String;

    fn next(&mut self) -> Option<Self::Item> {
        Some(format!("unnamed_{}", self.0.next()?))
    }
}

impl fmt::Debug for AutoNames {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("AutoNames").finish_non_exhaustive()
    }
}

impl Default for AutoNames {
    fn default() -> Self {
        Self(Box::new(crate::util::iter_uppercase_letter_names()))
    }
}

impl AutoNames {
    /// Returns the next unused autoname.
    pub fn next_unused<I: TypedIndex>(&mut self, names: &NameSpecBiMapBuilder<I>) -> String {
        self.find(|s| names.id_from_string(s).is_none())
            .expect("ran out of autonames")
    }
}
