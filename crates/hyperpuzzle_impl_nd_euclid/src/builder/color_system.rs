use eyre::{OptionExt, Result, ensure, eyre};
use hypermath::prelude::*;
use hyperpuzzle_core::prelude::*;
use indexmap::IndexMap;
use itertools::Itertools;

use super::{BadName, NameSet, NamingScheme};

const PUZZLE_PREFIX: &str = "puzzle:";

/// Sticker color during shape construction.
#[derive(Debug, Clone)]
pub struct ColorBuilder {}
impl ColorBuilder {}

/// Set of all sticker colors during shape construction.
#[derive(Debug, Default, Clone)]
pub struct ColorSystemBuilder {
    /// Color system ID.
    pub id: String,
    /// Name of the color system.
    pub name: Option<String>,

    /// Data for each color.
    by_id: PerColor<ColorBuilder>,
    /// User-specified color names.
    pub names: NamingScheme<Color>,

    /// Color schemes.
    pub schemes: IndexMap<String, PerColor<Option<DefaultColor>>>,
    /// Default color scheme.
    pub default_scheme: Option<String>,

    /// Orbits used to generate colors, tracked for puzzle dev purposes.
    pub color_orbits: Vec<DevOrbit<Color>>,

    /// Whether the color system has been modified.
    pub is_modified: bool,
    /// Whether the color system is shared (as opposed to ad-hoc defined for a
    /// single puzzle).
    pub is_shared: bool,
}
impl From<&ColorSystem> for ColorSystemBuilder {
    fn from(value: &ColorSystem) -> Self {
        let ColorSystem {
            id,
            name,
            list,
            schemes,
            default_scheme,
        } = value;

        let mut ret = ColorSystemBuilder::new_shared(id.clone());
        ret.name = Some(name.clone());
        for (i, color) in list {
            let names = itertools::chain([&color.name], &color.aliases);
            if let Err(e) = ret.get_or_add_with_name(NameSet::any(names), |_| ()) {
                log::error!("bad color system {id:?}: {e}");
            }
            ret.names
                .set_display(i, Some(color.display.clone()), |_| ());
        }
        ret.schemes = schemes
            .iter()
            .map(|(k, v)| (k.clone(), v.map_ref(|_, default| Some(default.clone()))))
            .collect();
        ret.default_scheme = Some(default_scheme.clone());

        // Reset the "is modified" flag.
        ret.is_modified = false;

        ret
    }
}
impl ColorSystemBuilder {
    /// Constructs a new shared color-system.
    pub fn new_shared(id: String) -> Self {
        Self {
            id,
            is_shared: true,
            ..Default::default()
        }
    }

    /// Constructs a new empty ad-hoc color system.
    pub fn new_ad_hoc(puzzle_id: &str) -> Self {
        Self {
            id: format!("{PUZZLE_PREFIX}{puzzle_id}"),
            is_shared: false,
            ..Default::default()
        }
    }

    /// Returns the name or the ID of the color system.
    pub fn display_name(&self) -> &str {
        self.name.as_deref().unwrap_or(&self.id)
    }

    /// Returns whether there are no colors in the color system.
    pub fn is_empty(&self) -> bool {
        self.by_id.is_empty()
    }
    /// Returns the number of colors in the color system.
    pub fn len(&self) -> usize {
        self.by_id.len()
    }

    /// Adds a new color.
    pub fn add(&mut self) -> Result<Color> {
        self.is_modified = true;

        let id = self.by_id.push(ColorBuilder {})?;
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

    /// Returns the name of the default color scheme.
    pub fn default_scheme_name(&self) -> &str {
        self.default_scheme
            .as_deref()
            .unwrap_or(hyperpuzzle_core::DEFAULT_COLOR_SCHEME_NAME)
    }

    /// Sets the default color for a single color.
    pub fn set_default_color(&mut self, id: Color, default_color: Option<DefaultColor>) {
        self.is_modified = true;

        let scheme = self
            .schemes
            .entry(self.default_scheme_name().to_owned())
            .or_default();
        scheme.extend_to_contain(id);
        scheme[id] = default_color;
    }
    /// Returns the default color for a single color, or `None` if it has not
    /// been set.
    pub fn get_default_color(&self, id: Color) -> Option<&DefaultColor> {
        self.schemes
            .get(self.default_scheme_name())?
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

    /// Returns the ID for the color with the given name, adding it to the color
    /// system if it does not already exist.
    pub fn get_or_add_with_name(
        &mut self,
        name: NameSet,
        warn_fn: impl Fn(BadName),
    ) -> Result<Color> {
        let s = name.canonical_name().ok_or_eyre(BadName::EmptySet)?;
        if let Some(&id) = self.names.names_to_ids().get(&s) {
            Ok(id)
        } else {
            let id = self.add()?;
            self.names.set_name(id, Some(name), warn_fn);
            Ok(id)
        }
    }

    /// Returns an iterator over all the colors, in the canonical ordering.
    pub fn iter(&self) -> impl Iterator<Item = (Color, &ColorBuilder)> {
        self.by_id.iter()
    }

    /// Validates and constructs a color system.
    ///
    /// Also returns a map from old color IDs to new color IDs.
    pub fn build(
        &self,
        puzzle_id: Option<&str>,
        dev_data: Option<&mut PuzzleDevData>,
        warn_fn: impl Copy + Fn(eyre::Report),
    ) -> Result<ColorSystem> {
        let mut id = self.id.clone();
        if self.is_shared {
            if self.is_modified {
                warn_fn(eyre!("shared color system cannot be modified"));
                if let Some(puzzle_id) = puzzle_id {
                    id = format!("{PUZZLE_PREFIX}{puzzle_id}");
                };
            }
            if self.name.is_none() {
                warn_fn(eyre!("color system has no name"));
            }
        } else {
            warn_fn(eyre!("using ad-hoc color system"));
        }
        let name = self.name.clone().unwrap_or_else(|| self.id.clone());

        let colors = super::iter_autonamed(
            self.len(),
            &self.names,
            hyperpuzzle_core::util::iter_uppercase_letter_names(),
        )
        .map(|(_id, (name_set, display))| {
            let mut string_set = name_set.string_set()?;
            ensure!(!string_set.is_empty(), "color is missing canonical name");
            eyre::Ok(ColorInfo {
                name: string_set.remove(0),
                aliases: string_set, // all except first
                display,
            })
        })
        .try_collect()?;

        let mut schemes: IndexMap<String, PerColor<DefaultColor>> = self
            .schemes
            .iter()
            .map(|(name, default_colors)| {
                let new_default_colors = default_colors.map_ref(|_, optional_default_color| {
                    optional_default_color
                        .clone()
                        .unwrap_or(DefaultColor::Unknown)
                });
                (name.clone(), new_default_colors)
            })
            .collect();

        let default_scheme = self.default_scheme_name().to_owned();
        if !schemes.contains_key(&default_scheme) {
            warn_fn(eyre!("missing default color scheme {default_scheme:?}"));
            schemes.insert(default_scheme.clone(), PerColor::new());
        }

        for (_name, list) in &mut schemes {
            list.resize(self.len())?;
            hyperpuzzle_core::ensure_color_scheme_is_valid(list.iter_values_mut(), |_| true);
        }

        if let Some(dev_data) = dev_data {
            dev_data.orbits.extend(
                self.color_orbits
                    .iter()
                    .map(|dev_orbit| dev_orbit.map(|i| Some(PuzzleElement::Color(i)))),
            );
        }

        let color_system = ColorSystem {
            id,
            name,

            list: colors,

            schemes,
            default_scheme,
        };
        Ok(color_system)
    }
}
