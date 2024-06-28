use eyre::{bail, eyre, Result};
use hypermath::prelude::*;
use indexmap::IndexMap;
use itertools::Itertools;

use super::{CustomOrdering, NamingScheme};
use crate::{Color, ColorInfo, DefaultColor, PerColor};

/// Sticker color during shape construction.
#[derive(Debug, Clone)]
pub struct ColorBuilder {
    surfaces: Vec<Hyperplane>,
}
impl ColorBuilder {
    /// Returns the color's surface set.
    pub fn surfaces(&self) -> &[Hyperplane] {
        &self.surfaces
    }
}

/// Set of all sticker colors during shape construction.
#[derive(Debug, Default, Clone)]
pub struct ColorSystemBuilder {
    /// Color data (not including name and ordering).
    by_id: PerColor<ColorBuilder>,
    /// Map from surface to color ID.
    surface_to_id: ApproxHashMap<Hyperplane, Color>,
    /// User-specified color names.
    pub names: NamingScheme<Color>,
    /// User-specified ordering of colors.
    pub ordering: CustomOrdering<Color>,

    /// Color schemes.
    pub schemes: IndexMap<String, PerColor<Option<DefaultColor>>>,
    /// Default color scheme.
    pub default_scheme: Option<String>,
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
    pub fn add(&mut self, surfaces: Vec<Hyperplane>) -> Result<Color> {
        // Check that the surfaces aren't already taken.
        for s in &surfaces {
            if self.surface_to_id.get(s).is_some() {
                bail!("surface is already taken");
            }
        }

        let id = self.by_id.push(ColorBuilder {
            surfaces: surfaces.clone(),
        })?;
        self.ordering.add(id)?;
        for s in surfaces {
            self.surface_to_id.insert(s, id);
        }

        Ok(id)
    }

    /// Adds a new color scheme.
    pub fn add_scheme(&mut self, name: String, mapping: PerColor<Option<DefaultColor>>) {
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
        self.by_id.get_mut(id)
    }

    /// Returns a map from surface to a color ID.
    pub fn surface_to_id(&self) -> &ApproxHashMap<Hyperplane, Color> {
        &self.surface_to_id
    }
    /// Returns a color ID from a set of surfaces.
    pub fn surface_set_to_id(&self, surfaces: &[Hyperplane]) -> Option<Color> {
        let mut colors_for_each_surface = surfaces.iter().filter_map(|s| self.surface_to_id.get(s));
        let &common_color = colors_for_each_surface.all_equal_value().ok()?;
        // Check that the resulting surface has the exact same number of colors.
        if self.by_id.get(common_color).ok()?.surfaces.len() == surfaces.len() {
            Some(common_color)
        } else {
            None
        }
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
        warn_fn: impl Copy + Fn(eyre::Report),
    ) -> Result<(
        PerColor<ColorInfo>,
        IndexMap<String, PerColor<Option<DefaultColor>>>,
        String,
    )> {
        let colors = super::iter_autonamed(
            &self.names,
            &self.ordering,
            crate::util::iter_uppercase_letter_names(),
            warn_fn,
        )
        .map(|(_id, (short_name, long_name))| {
            eyre::Ok(ColorInfo {
                short_name,
                long_name,
            })
        })
        .try_collect()?;

        let mut color_schemes = self.schemes.clone();
        for (_name, list) in &mut color_schemes {
            list.resize(self.len())?;
        }

        let default_color_scheme = self
            .default_scheme
            .clone()
            .unwrap_or(crate::DEFAULT_COLOR_SCHEME_NAME.to_string());
        if !color_schemes.contains_key(&default_color_scheme) {
            warn_fn(eyre!(
                "missing default color scheme {default_color_scheme:?}"
            ));
        }

        Ok((colors, color_schemes, default_color_scheme))
    }
}
