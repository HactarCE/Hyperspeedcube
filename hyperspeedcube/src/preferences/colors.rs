use std::collections::{btree_map, BTreeMap};

use hyperpuzzle::{ColorSystem, DefaultColor, Rgb};
use indexmap::IndexMap;
use itertools::Itertools;
use serde::{Deserialize, Serialize};

use crate::{preferences::DEFAULT_PREFS, L};

use super::{Preset, WithPresets};

pub type ColorScheme = IndexMap<String, DefaultColor>;

#[derive(Debug, Default, Display, EnumString, EnumIter, Copy, Clone, PartialEq, Eq, Hash)]

pub enum DefaultColorGradient {
    #[default]
    Rainbow,
    // Sinebow,
    // Turbo,
    // Spectral,
    // Cool,
    // Warm,
    // Plasma,
    // Viridis,
    // Cividis,
}
impl DefaultColorGradient {
    /// Returns the gradient as a [`colorous::Gradient`].
    pub fn to_colorous(self) -> colorous::Gradient {
        match self {
            Self::Rainbow => colorous::RAINBOW,
            // Self::Sinebow => colorous::SINEBOW,
            // Self::Turbo => colorous::TURBO,
            // Self::Spectral => colorous::SPECTRAL,
            // Self::Cool => colorous::COOL,
            // Self::Warm => colorous::WARM,
            // Self::Plasma => colorous::PLASMA,
            // Self::Viridis => colorous::VIRIDIS,
            // Self::Cividis => colorous::CIVIDIS,
        }
    }
    /// Samples the gradient at a point.
    pub fn sample(self, index: usize, total: usize) -> Rgb {
        let rgb = self.to_colorous().eval_rational(index, total).as_array();
        Rgb { rgb }
    }
    /// Returns a [`DefaultColor`] for the gradient
    pub fn default_color_at(self, index: usize, total: usize) -> DefaultColor {
        DefaultColor::Gradient {
            gradient_name: self.to_string(),
            index,
            total,
        }
    }
    pub fn default_color_at_end(self) -> DefaultColor {
        DefaultColor::Gradient {
            gradient_name: self.to_string(),
            index: usize::MAX,
            total: usize::MAX,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
#[serde(default)]
pub struct ColorPreferences {
    #[serde(flatten)]
    pub color_systems: BTreeMap<String, ColorSystemPreferences>,
    #[serde(skip)]
    empty_color_system: ColorSystemPreferences,
}
impl ColorPreferences {
    pub(super) fn post_init(&mut self) {
        for color_system_prefs in self.color_systems.values_mut() {
            color_system_prefs.post_init();
        }
    }

    pub fn color_system_mut(&mut self, color_system: &ColorSystem) -> &mut ColorSystemPreferences {
        match self.color_systems.entry(color_system.id.clone()) {
            btree_map::Entry::Vacant(e) => {
                e.insert(ColorSystemPreferences::from_color_system(color_system))
            }
            btree_map::Entry::Occupied(mut e) => {
                e.get_mut().update_builtin_schemes(color_system);
                e.into_mut()
            }
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
#[serde(default)]
pub struct GlobalColorPalette {
    pub custom_colors: IndexMap<String, Rgb>,
    pub builtin_colors: IndexMap<String, Rgb>,
    pub builtin_color_sets: IndexMap<String, Vec<Rgb>>,
}
impl GlobalColorPalette {
    pub(super) fn post_init(&mut self) {
        self.builtin_colors = DEFAULT_PREFS
            .color_palette
            .builtin_colors
            .iter()
            .map(|(k, v)| (k.clone(), self.builtin_colors.get(k).unwrap_or(v).clone()))
            .collect();

        self.builtin_color_sets = DEFAULT_PREFS
            .color_palette
            .builtin_color_sets
            .iter()
            .map(|(k, v)| {
                let user_value = self
                    .builtin_color_sets
                    .get(k)
                    .unwrap_or(const { &Vec::new() });
                (
                    k.clone(),
                    v.iter()
                        .enumerate()
                        .map(|(i, v)| user_value.get(i).unwrap_or(v).clone())
                        .collect(),
                )
            })
            .collect();
    }

    pub fn has(&self, color: &DefaultColor) -> bool {
        match color {
            // Skip sampling the gradient
            DefaultColor::Gradient { gradient_name, .. } => {
                gradient_name.parse::<DefaultColorGradient>().is_ok()
            }

            // No way to make the other cases faster
            _ => self.get(color).is_some(),
        }
    }

    pub fn get_set(&self, set_name: &str) -> Option<&Vec<Rgb>> {
        self.builtin_color_sets.get(set_name)
    }

    pub fn get(&self, color: &DefaultColor) -> Option<Rgb> {
        match color {
            DefaultColor::Unknown => None,
            DefaultColor::HexCode { rgb } => Some(*rgb),
            DefaultColor::Single { name } => None
                .or_else(|| self.builtin_colors.get(name))
                .or_else(|| self.custom_colors.get(name))
                .copied(),
            DefaultColor::Set { set_name, index } => self
                .get_set(set_name)
                .and_then(|set| set.get(*index))
                .copied(),
            DefaultColor::Gradient {
                gradient_name,
                index,
                total,
            } => {
                let gradient = gradient_name.parse::<DefaultColorGradient>().ok()?;
                Some(gradient.sample(*index, *total))
            }
        }
    }

    /// Modfies a color scheme if necessary to ensure that it is valid for its
    /// color system and the current global palette. Returns `true` if it was
    /// modified.
    #[must_use]
    pub fn ensure_color_scheme_is_valid_for_color_system(
        &self,
        scheme: &mut ColorScheme,
        color_system: &ColorSystem,
    ) -> bool {
        let mut changed = false;

        let names_match = itertools::equal(
            scheme.iter().map(|(k, _v)| k),
            color_system.list.iter().map(|(_id, color)| &color.name),
        );
        if !names_match {
            changed = true;
            *scheme = color_system
                .list
                .iter_values()
                .map(|color| {
                    scheme
                        .swap_remove_entry(&color.name)
                        .unwrap_or_else(|| (color.name.clone(), DefaultColor::Unknown))
                })
                .collect();
        }

        changed |= hyperpuzzle::ensure_color_scheme_is_valid(scheme.values_mut(), |c| self.has(c));

        changed
    }

    pub fn groups_of_sets(&self) -> Vec<(String, Vec<(&String, &[Rgb])>)> {
        self.builtin_color_sets
            .iter()
            .sorted_by_key(|(_, colors)| colors.len())
            .group_by(|(_, colors)| colors.len())
            .into_iter()
            .map(|(value, sets)| {
                let group_name = match value {
                    1 => L.colors.set_sizes._1.to_string(),
                    2 => L.colors.set_sizes._2.to_string(),
                    3 => L.colors.set_sizes._3.to_string(),
                    4 => L.colors.set_sizes._4.to_string(),
                    5 => L.colors.set_sizes._5.to_string(),
                    6 => L.colors.set_sizes._6.to_string(),
                    7 => L.colors.set_sizes._7.to_string(),
                    8 => L.colors.set_sizes._8.to_string(),
                    9 => L.colors.set_sizes._9.to_string(),
                    10 => L.colors.set_sizes._10.to_string(),
                    n => L.colors.set_sizes.n.with(&n.to_string()),
                };
                (
                    group_name,
                    sets.map(|(name, rgbs)| (name, rgbs.as_slice())).collect(),
                )
            })
            .collect()
    }
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct ColorSystemPreferences {
    pub schemes: WithPresets<ColorScheme>,
}
impl ColorSystemPreferences {
    fn post_init(&mut self) {
        for scheme in &mut self.schemes.user {
            hyperpuzzle::ensure_color_scheme_is_valid(scheme.value.values_mut(), |_| true);
        }

        self.schemes.post_init(None);
    }

    /// Creates factory-default preferences for a color system.
    pub fn from_color_system(color_system: &ColorSystem) -> Self {
        let mut ret = Self::default();
        let _changed = ret.update_builtin_schemes(color_system);
        ret.schemes.load_preset(&color_system.default_scheme);
        ret
    }
    /// Updates the built-in schemes for the color system.
    ///
    /// Deletes any user color schemes with the same name.
    pub fn update_builtin_schemes(&mut self, color_system: &ColorSystem) {
        self.schemes.set_builtin_presets(
            color_system
                .schemes
                .keys()
                .map(|name| preset_from_color_scheme(color_system, name))
                .collect(),
        );
    }
}

fn preset_from_color_scheme(color_system: &ColorSystem, name: &str) -> Preset<ColorScheme> {
    let value = color_system
        .get_scheme_or_default(name)
        .iter()
        .map(|(id, default_color)| (color_system.list[id].name.clone(), default_color.clone()))
        .collect();
    Preset {
        name: name.to_string(),
        value,
    }
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
#[serde(default)]
pub struct SavedCustomColor {
    pub name: String,
    pub rgb: Rgb,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
#[serde(default)]
pub struct SavedCustomColorSet {
    pub name: String,
    pub colors: Vec<Rgb>,
}
