use eyre::{bail, Result};
use hypermath::prelude::*;
use itertools::Itertools;

use super::{CustomOrdering, NamingScheme};
use crate::{Color, ColorInfo, PerColor};

/// Sticker color during shape construction.
#[derive(Debug, Clone)]
pub struct ColorBuilder {
    surfaces: Vec<Hyperplane>,

    /// Default color string.
    pub default_color: Option<String>,
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
            default_color: None,
        })?;
        self.ordering.add(id)?;
        for s in surfaces {
            self.surface_to_id.insert(s, id);
        }

        Ok(id)
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
    pub fn build(&self) -> Result<PerColor<ColorInfo>> {
        super::iter_autonamed(
            &self.names,
            &self.ordering,
            crate::util::iter_uppercase_letter_names(),
        )
        .map(|(id, name)| {
            let default_color = self.get(id)?.default_color.clone();
            eyre::Ok(ColorInfo {
                name,
                default_color,
            })
        })
        .try_collect()
    }
}
