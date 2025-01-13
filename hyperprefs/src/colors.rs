use std::collections::{btree_map, BTreeMap};

use hyperpuzzle::{ColorSystem, DefaultColor, Rgb};
use indexmap::IndexMap;
use itertools::Itertools;

use super::{schema, PresetsList, DEFAULT_PREFS_RAW};

pub type ColorScheme = IndexMap<String, DefaultColor>;

#[derive(Debug, Default)]
pub struct ColorSchemePreferences(BTreeMap<String, ColorSystemPreferences>);
impl schema::PrefsConvert for ColorSchemePreferences {
    type DeserContext = ();
    type SerdeFormat = BTreeMap<String, schema::current::ColorSystemPreferences>;

    fn to_serde(&self) -> Self::SerdeFormat {
        self.0
            .iter()
            .map(|(k, v)| (k.clone(), v.to_serde()))
            .collect()
    }
    fn reload_from_serde(&mut self, ctx: &Self::DeserContext, value: Self::SerdeFormat) {
        schema::reload_btreemap(&mut self.0, ctx, value);
    }
}
impl ColorSchemePreferences {
    pub fn get(&self, color_system: &ColorSystem) -> Option<&ColorSystemPreferences> {
        self.0.get(&color_system.id)
    }
    pub fn get_mut(&mut self, color_system: &ColorSystem) -> &mut ColorSystemPreferences {
        match self.0.entry(color_system.id.clone()) {
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

#[derive(Debug, Default, Display, EnumString, EnumIter, Copy, Clone, PartialEq, Eq, Hash)]
pub enum DefaultColorGradient {
    #[strum(serialize = "Light Rainbow")]
    LightRainbow,
    #[default]
    Rainbow,
    #[strum(serialize = "Dark Rainbow")]
    DarkRainbow,
    #[strum(serialize = "Darker Rainbow")]
    DarkerRainbow,
    Turbo,
    Spectral,
    Cool,
    Warm,
    Plasma,
    Viridis,
    Cividis,
}
impl DefaultColorGradient {
    /// Returns whether the gradient is cyclic (ends at the same place it
    /// starts).
    fn is_cyclic(self) -> bool {
        use DefaultColorGradient::*;
        match self {
            LightRainbow | Rainbow | DarkRainbow | DarkerRainbow => true,
            _ => false,
        }
    }
    /// Samples the gradient at `index` in the range `0..=total`.
    pub fn eval_rational(self, index: usize, mut total: usize) -> Rgb {
        if total == 0 {
            total = 1;
        }

        let limit = match self.is_cyclic() {
            true => total,      // Exclude endpoints on cyclic gradient
            false => total - 1, // Include endpoints on non-cyclic gradient
        };

        self.eval_continuous(index.clamp(0, limit) as f64 / total as f64)
    }
    /// Samples the gradient at a point from 0.0 to 1.0.
    pub fn eval_continuous(self, t: f64) -> Rgb {
        fn eval(g: colorous::Gradient, t: f64) -> Rgb {
            let rgb = g.eval_continuous(t).as_array();
            Rgb { rgb }
        }

        match self {
            Self::Rainbow => eval(colorous::SINEBOW, t),
            Self::Turbo => eval(colorous::TURBO, t),
            Self::Spectral => eval(colorous::SPECTRAL, t),
            Self::Cool => eval(colorous::COOL, t),
            Self::Warm => eval(colorous::WARM, t),
            Self::Plasma => eval(colorous::PLASMA, t),
            Self::Viridis => eval(colorous::VIRIDIS, t),
            Self::Cividis => eval(colorous::CIVIDIS, t),

            Self::LightRainbow => {
                let rainbow = Self::Rainbow.eval_continuous(t);
                hyperpuzzle::Rgb::mix(Rgb::WHITE, rainbow, 0.9)
            }
            Self::DarkRainbow => {
                let rainbow = Self::Rainbow.eval_continuous(t);
                hyperpuzzle::Rgb::mix(Rgb::BLACK, rainbow, 0.4)
            }
            Self::DarkerRainbow => {
                let rainbow = Self::Rainbow.eval_continuous(t);
                hyperpuzzle::Rgb::mix(Rgb::BLACK, rainbow, 0.2)
            }
        }
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

#[derive(Debug, Default)]
pub struct GlobalColorPalette {
    pub custom_colors: PresetsList<Rgb>,
    pub builtin_colors: IndexMap<String, Rgb>,
    pub builtin_color_sets: IndexMap<String, Vec<Rgb>>,
}
impl schema::PrefsConvert for GlobalColorPalette {
    type DeserContext = ();
    type SerdeFormat = schema::current::GlobalColorPalette;

    fn to_serde(&self) -> Self::SerdeFormat {
        let Self {
            custom_colors,
            builtin_colors,
            builtin_color_sets,
        } = self;

        schema::current::GlobalColorPalette {
            custom_colors: custom_colors.to_serde_map(),
            builtin_colors: builtin_colors.clone(),
            builtin_color_sets: builtin_color_sets.clone(),
        }
    }
    fn reload_from_serde(&mut self, ctx: &Self::DeserContext, value: Self::SerdeFormat) {
        let schema::current::GlobalColorPalette {
            custom_colors,
            builtin_colors,
            builtin_color_sets,
        } = value;

        self.custom_colors.reload_from_serde_map(ctx, custom_colors);

        self.builtin_colors = DEFAULT_PREFS_RAW
            .color_palette
            .builtin_colors
            .iter()
            .map(|(k, v)| (k.clone(), *builtin_colors.get(k).unwrap_or(v)))
            .collect();

        self.builtin_color_sets = DEFAULT_PREFS_RAW
            .color_palette
            .builtin_color_sets
            .iter()
            .map(|(k, v)| {
                let user_value = builtin_color_sets.get(k).unwrap_or(const { &Vec::new() });
                (
                    k.clone(),
                    v.iter()
                        .enumerate()
                        .map(|(i, v)| *user_value.get(i).unwrap_or(v))
                        .collect(),
                )
            })
            .collect();
    }
}
impl GlobalColorPalette {
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
                .or_else(|| Some(&self.custom_colors.get(name)?.value))
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
                Some(gradient.eval_rational(*index, *total))
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

    #[allow(clippy::type_complexity)]
    pub fn groups_of_sets(
        &self,
        get_group_name: impl Fn(usize) -> String,
    ) -> Vec<(String, Vec<(&String, &[Rgb])>)> {
        self.builtin_color_sets
            .iter()
            .sorted_by_key(|(_, colors)| colors.len())
            .chunk_by(|(_, colors)| colors.len())
            .into_iter()
            .map(|(value, sets)| {
                (
                    get_group_name(value),
                    sets.map(|(name, rgbs)| (name, rgbs.as_slice())).collect(),
                )
            })
            .collect()
    }
}

#[derive(Debug, Default)]
pub struct ColorSystemPreferences {
    pub schemes: PresetsList<ColorScheme>,
}
impl schema::PrefsConvert for ColorSystemPreferences {
    type DeserContext = ();
    type SerdeFormat = schema::current::ColorSystemPreferences;

    fn to_serde(&self) -> Self::SerdeFormat {
        let Self { schemes } = self;

        schemes
            .user_presets()
            .map(|preset| (preset.name().clone(), preset.value.clone()))
            .collect()
    }
    fn reload_from_serde(&mut self, ctx: &Self::DeserContext, value: Self::SerdeFormat) {
        self.schemes.reload_from_serde_map(ctx, value);
    }
}
impl ColorSystemPreferences {
    /// Creates factory-default preferences for a color system.
    pub fn from_color_system(color_system: &ColorSystem) -> Self {
        let mut ret = Self::default();
        ret.update_builtin_schemes(color_system);
        ret.schemes
            .set_last_loaded(color_system.default_scheme.clone());
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

fn preset_from_color_scheme(color_system: &ColorSystem, name: &str) -> (String, ColorScheme) {
    let value = color_system
        .get_scheme_or_default(name)
        .iter()
        .map(|(id, default_color)| (color_system.list[id].name.clone(), default_color.clone()))
        .collect();
    (name.to_string(), value)
}
