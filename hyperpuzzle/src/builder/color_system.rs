use eyre::{bail, eyre, Result};
use hypermath::prelude::*;
use indexmap::IndexMap;
use itertools::Itertools;

use super::{CustomOrdering, NamingScheme};
use crate::{Color, ColorInfo, ColorSystem, DefaultColor, PerColor};

const PUZZLE_PREFIX: &str = "puzzle:";

/// Sticker color during shape construction.
#[derive(Debug, Clone)]
pub struct ColorBuilder {}
impl ColorBuilder {}

/// Set of all sticker colors during shape construction.
#[derive(Debug, Default, Clone)]
pub struct ColorSystemBuilder {
    /// Color system ID.
    pub id: Option<String>,
    /// Name of the color system.
    pub name: Option<String>,

    /// Data for each color.
    by_id: PerColor<ColorBuilder>,
    /// User-specified color names.
    pub names: NamingScheme<Color>,
    /// User-specified ordering of colors.
    pub ordering: CustomOrdering<Color>,

    /// Color schemes.
    pub schemes: IndexMap<String, PerColor<Option<DefaultColor>>>,
    /// Default color scheme.
    pub default_scheme: Option<String>,

    /// Whether the color system has been modified.
    pub is_modified: bool,
}
impl ColorSystemBuilder {
    /// Constructs a new empty color system.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the number of colors in the color system.
    pub fn len(&self) -> usize {
        self.by_id.len()
    }

    /// Adds a new color.
    pub fn add(&mut self) -> Result<Color> {
        self.is_modified = true;

        let id = self.by_id.push(ColorBuilder {})?;
        self.ordering.add(id)?;
        Ok(id)
    }

    /// Adds a new color scheme.
    pub fn add_scheme(&mut self, name: String, mapping: PerColor<Option<DefaultColor>>) {
        self.is_modified = true;

        match self.schemes.entry(name) {
            indexmap::map::Entry::Occupied(mut e) => {
                for (id, c) in mapping {
                    e.get_mut().extend_to_contain(id);
                    if let Some(new_default_color) = c {
                        e.get_mut()[id] = Some(new_default_color);
                    }
                }
            }
            indexmap::map::Entry::Vacant(e) => {
                e.insert(mapping);
            }
        }
    }

    /// Sets the default color for a single color.
    pub fn set_default_color(&mut self, id: Color, default_color: Option<DefaultColor>) {
        self.is_modified = true;

        let scheme = self
            .schemes
            .entry(crate::DEFAULT_COLOR_SCHEME_NAME.to_owned())
            .or_default();
        scheme.extend_to_contain(id);
        scheme[id] = default_color;
    }
    /// Returns the default color for a single color, or `None` if it has not
    /// been set.
    pub fn get_default_color(&self, id: Color) -> Option<&DefaultColor> {
        self.schemes
            .get(&crate::DEFAULT_COLOR_SCHEME_NAME.to_string())?
            .get(id)
            .ok()?
            .as_ref()
    }

    /// Returns a reference to a color by ID, or an error if the ID is out of
    /// range.
    pub fn get(&self, id: Color) -> Result<&ColorBuilder, IndexOutOfRange> {
        self.by_id.get(id)
    }
    /// Returns a mutable reference to a color by ID, or an error if the ID is
    /// out of range.
    pub fn get_mut(&mut self, id: Color) -> Result<&mut ColorBuilder, IndexOutOfRange> {
        self.is_modified = true;

        self.by_id.get_mut(id)
    }

    /// Returns an iterator over all the colors, in the canonical ordering.
    pub fn iter(&self) -> impl Iterator<Item = (Color, &ColorBuilder)> {
        self.ordering
            .ids_in_order()
            .iter()
            .map(|&id| (id, &self.by_id[id]))
    }

    /// Validates and constructs a color system.
    pub fn build(
        &self,
        puzzle_id: &str,
        warn_fn: impl Copy + Fn(eyre::Report),
    ) -> Result<ColorSystem> {
        if self.id.is_some() && self.is_modified {
            bail!("shared color system cannot be modified");
        }
        let id = self
            .id
            .clone()
            .unwrap_or_else(|| format!("{PUZZLE_PREFIX}{puzzle_id}"));
        let name = self
            .name
            .clone()
            .unwrap_or_else(|| crate::util::titlecase(self.id.as_deref().unwrap_or(puzzle_id)));

        let colors = super::iter_autonamed(
            &self.names,
            &self.ordering,
            crate::util::iter_uppercase_letter_names(),
            warn_fn,
        )
        .map(|(_id, (name, display))| eyre::Ok(ColorInfo { name, display }))
        .try_collect()?;

        let mut schemes = self.schemes.clone();
        for (_name, list) in &mut schemes {
            list.resize(self.len())?;
        }

        let default_scheme = self
            .default_scheme
            .clone()
            .unwrap_or(crate::DEFAULT_COLOR_SCHEME_NAME.to_string());
        if !schemes.contains_key(&default_scheme) {
            warn_fn(eyre!("missing default color scheme {default_scheme:?}"));
        }

        Ok(ColorSystem {
            id,
            name,

            list: colors,

            schemes,
            default_scheme,
        })
    }
}
