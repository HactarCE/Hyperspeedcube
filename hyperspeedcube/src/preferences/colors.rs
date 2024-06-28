use hyperpuzzle::{DefaultColor, Rgb};
use indexmap::IndexMap;
use itertools::Itertools;
use serde::{Deserialize, Serialize};

use crate::preferences::DEFAULT_PREFS;

#[derive(Serialize, Deserialize, Debug, Default, Clone, PartialEq, Eq)]
#[serde(default)]
pub struct ColorPreferences {
    pub singles: IndexMap<String, Rgb>,
    pub sets: IndexMap<String, Vec<Rgb>>,

    pub custom_singles: IndexMap<String, Rgb>,
    pub custom_sets: IndexMap<String, Vec<Rgb>>,
}
impl ColorPreferences {
    pub fn canonicalize(&mut self) {
        *self = Self {
            singles: DEFAULT_PREFS
                .colors
                .singles
                .iter()
                .map(|(k, v)| (k.clone(), self.singles.get(k).unwrap_or(v).clone()))
                .collect(),
            sets: DEFAULT_PREFS
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
                .collect(),

            custom_singles: self.custom_singles.clone(),
            custom_sets: self.custom_sets.clone(),
        };
    }

    pub fn get(&self, color: &DefaultColor) -> Option<Rgb> {
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

    // pub fn groups_of_sets(
    //     &self,
    // ) -> itertools::GroupBy<
    //     usize,
    //     indexmap::map::Iter<'_, String, Vec<Rgb>>,
    //     fn(&(&String, &Vec<Rgb>)) -> usize,
    // > {
    //     self.sets.iter().group_by(|(_, colors)| colors.len())
    // }

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
