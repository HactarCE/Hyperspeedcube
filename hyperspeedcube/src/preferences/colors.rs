use std::collections::{btree_map, BTreeMap};

use hyperpuzzle::{ColorSystem, DefaultColor, Rgb};
use indexmap::IndexMap;
use itertools::Itertools;
use serde::{Deserialize, Serialize};

use crate::preferences::DEFAULT_PREFS;

use super::{Preset, WithPresets};

pub type ColorScheme = IndexMap<String, Option<DefaultColor>>;

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
#[serde(default)]
pub struct ColorPreferences {
    pub singles: IndexMap<String, Rgb>,
    pub sets: IndexMap<String, Vec<Rgb>>,

    pub custom_singles: IndexMap<String, Rgb>,
    pub custom_sets: IndexMap<String, Vec<Rgb>>,

    pub color_systems: BTreeMap<String, ColorSystemPreferences>,
    #[serde(skip)]
    empty_color_system: ColorSystemPreferences,
}
impl ColorPreferences {
    pub(super) fn post_init(&mut self) {
        self.singles = DEFAULT_PREFS
            .colors
            .singles
            .iter()
            .map(|(k, v)| (k.clone(), self.singles.get(k).unwrap_or(v).clone()))
            .collect();

        self.sets = DEFAULT_PREFS
            .colors
            .sets
            .iter()
            .map(|(k, v)| {
                let user_value = self.sets.get(k).unwrap_or(const { &Vec::new() });
                (
                    k.clone(),
                    v.iter()
                        .enumerate()
                        .map(|(i, v)| user_value.get(i).unwrap_or(v).clone())
                        .collect(),
                )
            })
            .collect();

        for color_system_prefs in self.color_systems.values_mut() {
            color_system_prefs.post_init();
        }
    }

    pub fn get_color(&self, color: &DefaultColor) -> Option<Rgb> {
        match color {
            DefaultColor::HexCode { rgb } => Some(*rgb),
            DefaultColor::Single { name } => None
                .or_else(|| self.singles.get(name))
                .or_else(|| self.custom_singles.get(name))
                .copied(),
            DefaultColor::Set { set_name, index } => None
                .or_else(|| self.sets.get(set_name)?.get(*index))
                .or_else(|| self.custom_sets.get(set_name)?.get(*index))
                .copied(),
        }
    }

    pub fn groups_of_sets(&self) -> Vec<(String, Vec<(&String, &[Rgb])>)> {
        self.sets
            .iter()
            .group_by(|(_, colors)| colors.len())
            .into_iter()
            .map(|(value, sets)| {
                let group_name = match value {
                    1 => "Monads".to_string(),
                    2 => "Dyads".to_string(),
                    3 => "Triads".to_string(),
                    4 => "Tetrads".to_string(),
                    5 => "Pentads".to_string(),
                    6 => "Hexads".to_string(),
                    7 => "Heptads".to_string(),
                    8 => "Octads".to_string(),
                    9 => "Nonads".to_string(),
                    10 => "Decads".to_string(),
                    n => format!("{n}-ads"),
                };
                (
                    group_name,
                    sets.map(|(name, rgbs)| (name, rgbs.as_slice())).collect(),
                )
            })
            .collect()
    }

    pub fn color_system_mut(&mut self, color_system: &ColorSystem) -> &mut ColorSystemPreferences {
        if color_system.id.is_empty() {
            &mut self.empty_color_system // don't save this in the `color_systems` map.
        } else {
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
    pub fn current_color_scheme(&self, color_system: &ColorSystem) -> Preset<ColorScheme> {
        match self.color_systems.get(&color_system.id) {
            Some(color_system_prefs) => color_system_prefs.schemes.current_preset(),
            None => preset_from_color_scheme(color_system, &color_system.default_scheme),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct ColorSystemPreferences {
    pub schemes: WithPresets<ColorScheme>,
}
impl ColorSystemPreferences {
    fn post_init(&mut self) {
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
