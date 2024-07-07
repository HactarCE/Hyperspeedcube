use std::borrow::Cow;

use indexmap::IndexMap;

use super::*;

/// System of sticker colors for a puzzle.
#[derive(Debug)]
pub struct ColorSystem {
    /// ID, which may indicate that the color system is shared across multiple
    /// puzzles.
    pub id: String,
    /// Human-friendly name for the color system.
    pub name: String,

    /// List of colors, indexed by ID.
    pub list: PerColor<ColorInfo>,

    /// List of named color schemes.
    pub schemes: IndexMap<String, PerColor<Option<DefaultColor>>>,
    /// Name of the default color scheme, which is typically `"Default"`.
    pub default_scheme: String,
}
impl ColorSystem {
    /// Returns the default color scheme.
    pub fn default_scheme(&self) -> Cow<'_, PerColor<Option<DefaultColor>>> {
        match self.schemes.get(&self.default_scheme) {
            Some(scheme) => Cow::Borrowed(scheme),
            None => {
                let mut ret = PerColor::new();
                ret.resize(self.list.len()).expect("impossible overflow!");
                Cow::Owned(ret)
            }
        }
    }
    /// Returns the color scheme with the given name, or the default scheme if
    /// it doesn't exist.
    pub fn get_scheme_or_default(&self, name: &str) -> Cow<'_, PerColor<Option<DefaultColor>>> {
        match self.schemes.get(name) {
            Some(scheme) => Cow::Borrowed(scheme),
            None => self.default_scheme(),
        }
    }

    /// Returns an empty color system.
    pub fn new_empty() -> Self {
        Self {
            id: String::new(),
            name: String::new(),
            list: PerColor::new(),
            schemes: IndexMap::new(),
            default_scheme: String::new(),
        }
    }

    /// Returns the number of colors in the color system.
    pub fn len(&self) -> usize {
        self.list.len()
    }
}
