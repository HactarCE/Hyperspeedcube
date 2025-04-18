use std::borrow::Cow;
use std::collections::{HashMap, HashSet};

use indexmap::IndexMap;

use super::*;
use crate::NameSpecBiMap;

/// System of sticker colors for a puzzle.
#[derive(Debug)]
pub struct ColorSystem {
    /// Color system ID.
    pub id: String,
    /// Human-friendly name for the color system.
    pub name: String,

    /// Color names.
    pub names: NameSpecBiMap<Color>,
    /// Color display names.
    pub display_names: PerColor<String>,

    /// List of named color schemes.
    pub schemes: IndexMap<String, PerColor<DefaultColor>>,
    /// Name of the default color scheme, which is typically `"Default"`.
    pub default_scheme: String,

    /// Orbits used to generate colors.
    pub orbits: Vec<Orbit<Color>>,
}
impl ColorSystem {
    /// Returns a rainbow color scheme with the given length.
    fn new_rainbow_scheme(len: usize) -> PerColor<DefaultColor> {
        (0..len)
            .map(|i| DefaultColor::Gradient {
                gradient_name: crate::DEFAULT_COLOR_GRADIENT_NAME.to_string(),
                index: i,
                total: len,
            })
            .collect()
    }

    /// Returns the default color scheme.
    pub fn default_scheme(&self) -> Cow<'_, PerColor<DefaultColor>> {
        match self.schemes.get(&self.default_scheme) {
            Some(scheme) => Cow::Borrowed(scheme),
            None => Cow::Owned(Self::new_rainbow_scheme(self.len())),
        }
    }
    /// Returns the color scheme with the given name, or the default scheme if
    /// it doesn't exist.
    pub fn get_scheme_or_default(&self, name: &str) -> Cow<'_, PerColor<DefaultColor>> {
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
            names: NameSpecBiMap::new(),
            display_names: PerColor::new(),
            schemes: IndexMap::new(),
            default_scheme: String::new(),
            orbits: vec![],
        }
    }

    /// Returns whether there are no colors in the color system.
    pub fn is_empty(&self) -> bool {
        self.names.is_empty()
    }
    /// Returns the number of colors in the color system.
    pub fn len(&self) -> usize {
        self.names.len()
    }
}

/// Modifies a color scheme to ensure that it is valid. Namely that:
///
/// - There are no unknown colors.
/// - Each color is used at most once.
/// - Colors taken from a gradient are evenly spaced and use the whole gradient.
///
/// Returns whether the color scheme was modified.
pub fn ensure_color_scheme_is_valid<'a>(
    scheme: impl IntoIterator<Item = &'a mut DefaultColor>,
    mut is_valid_color: impl FnMut(&DefaultColor) -> bool,
) -> bool {
    let mut changed = false;

    #[derive(Debug)]
    struct GradientMut<'a> {
        index: &'a mut usize,
        total: &'a mut usize,
    }

    let mut gradient_to_color_list = HashMap::<String, Vec<GradientMut<'a>>>::new();
    let mut used_colors = HashSet::new();

    // Sort the colors into three buckets.
    for c in scheme {
        match c {
            // If the color hasn't been used yet, keep it.
            DefaultColor::HexCode { .. }
            | DefaultColor::Single { .. }
            | DefaultColor::Set { .. }
                if is_valid_color(c) && used_colors.insert(c.clone()) => {}

            // If the color is unknown or has already been used, add it to the
            // end of the default color gradient.
            DefaultColor::Unknown
            | DefaultColor::HexCode { .. }
            | DefaultColor::Single { .. }
            | DefaultColor::Set { .. } => {
                changed = true;
                *c = DefaultColor::Gradient {
                    gradient_name: crate::DEFAULT_COLOR_GRADIENT_NAME.to_string(),
                    index: usize::MAX,
                    total: usize::MAX,
                };

                let DefaultColor::Gradient {
                    gradient_name: _,
                    index,
                    total,
                } = c
                else {
                    unreachable!()
                };

                gradient_to_color_list
                    .entry(crate::DEFAULT_COLOR_GRADIENT_NAME.to_string())
                    .or_default()
                    .push(GradientMut { index, total });
            }

            // If the color is a gradient, then add it in its place to the
            // gradient.
            DefaultColor::Gradient {
                gradient_name,
                index,
                total,
            } => {
                gradient_to_color_list
                    .entry(gradient_name.clone())
                    .or_default()
                    .push(GradientMut { index, total });
            }
        }
    }

    // Ensure each gradient is in order.
    for (_gradient_name, mut colors) in gradient_to_color_list {
        // Sort by desired index.
        colors.sort_by_key(|c| *c.index);

        let total = colors.len();
        for (i, color) in colors.into_iter().enumerate() {
            if *color.index != i {
                changed = true;
                *color.index = i;
            }
            if *color.total != total {
                changed = true;
                *color.total = total;
            }
        }
    }

    changed
}
