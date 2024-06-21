//! User preferences.
//!
//! For a list of key names, see `VirtualKeyCode` in this file:
//! https://github.com/rust-windowing/winit/blob/master/src/event.rs

use std::{
    fmt::write,
    ops::{Index, IndexMut},
    path::PathBuf,
};

use bitvec::vec::BitVec;
use hyperpuzzle::Puzzle;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::fmt;

mod gfx;
mod info;
mod interaction;
mod keybinds;
// mod migration; // TODO
mod migration {
    pub const LATEST_VERSION: u32 = 2;
}
mod mousebinds;
#[cfg(not(target_arch = "wasm32"))]
mod persist_local;
#[cfg(target_arch = "wasm32")]
mod persist_web;
mod styles;
mod view;

pub use gfx::*;
pub use info::*;
pub use interaction::*;
pub use keybinds::*;
pub use mousebinds::*;
#[cfg(not(target_arch = "wasm32"))]
use persist_local as persist;
#[cfg(target_arch = "wasm32")]
use persist_web as persist;
pub use styles::*;
pub use view::*;

use crate::commands::{Command, PuzzleCommand, PuzzleMouseCommand};

const PREFS_FILE_FORMAT: config::FileFormat = config::FileFormat::Yaml;
const DEFAULT_PREFS_STR: &str = include_str!("default.yaml");

lazy_static! {
    pub static ref DEFAULT_PREFS: Preferences =
        serde_yaml::from_str(DEFAULT_PREFS_STR).expect("error loading default preferences");
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
#[serde(default)]
pub struct Preferences {
    #[serde(skip)]
    pub needs_save: bool,
    #[serde(skip)]
    pub needs_save_eventually: bool,

    /// Preferences file format version.
    #[serde(skip_deserializing)]
    pub version: u32,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub log_file: Option<PathBuf>,

    pub show_welcome_at_startup: bool,

    pub info: InfoPreferences,

    pub gfx: GfxPreferences,
    pub interaction: InteractionPreferences,
    pub styles: StylePreferences,

    pub view_3d: WithPresets<ViewPreferences>,
    pub view_4d: WithPresets<ViewPreferences>,
    #[serde(skip)]
    pub latest_view_prefs_set: PuzzleViewPreferencesSet,

    pub piece_filters: (), // TODO

    pub global_keybinds: Vec<Keybind<Command>>,
    pub puzzle_keybinds: (), // TODO
    pub mousebinds: Vec<Mousebind<PuzzleMouseCommand>>,
}
impl Index<PuzzleViewPreferencesSet> for Preferences {
    type Output = WithPresets<ViewPreferences>;

    fn index(&self, index: PuzzleViewPreferencesSet) -> &Self::Output {
        match index {
            PuzzleViewPreferencesSet::Dim3D => &self.view_3d,
            PuzzleViewPreferencesSet::Dim4D => &self.view_4d,
        }
    }
}
impl IndexMut<PuzzleViewPreferencesSet> for Preferences {
    fn index_mut(&mut self, index: PuzzleViewPreferencesSet) -> &mut Self::Output {
        match index {
            PuzzleViewPreferencesSet::Dim3D => &mut self.view_3d,
            PuzzleViewPreferencesSet::Dim4D => &mut self.view_4d,
        }
    }
}
impl Preferences {
    pub fn load(backup: Option<&Self>) -> Self {
        let mut config = config::Config::builder();

        // Load default preferences.
        let default_config_source = config::File::from_str(DEFAULT_PREFS_STR, PREFS_FILE_FORMAT);
        config = config.add_source(default_config_source.clone());

        // Load user preferences.
        match persist::user_config_source() {
            Ok(config_source) => config = config.add_source(config_source),
            Err(e) => log::warn!("Error loading user preferences: {}", e),
        }

        config
            .build()
            .and_then(|c| c.try_deserialize())
            // .and_then(migration::try_deserialize) // TODO: migration
            .unwrap_or_else(|e| {
                log::warn!("Error loading preferences: {}", e);

                persist::backup_prefs_file();

                // Try backup
                backup
                    .cloned()
                    // Try just default config
                    .or_else(|| {
                        config::Config::builder()
                            .add_source(default_config_source)
                            .build()
                            .ok()?
                            .try_deserialize()
                            .ok()
                    })
                    .unwrap_or_default()
            })
    }

    pub fn save(&mut self) {
        self.needs_save = false;

        // Set version number.
        self.version = migration::LATEST_VERSION;

        let prefs = self.clone();
        std::thread::spawn(move || {
            let result = persist::save(&prefs);

            match result {
                Ok(()) => log::debug!("Saved preferences"),
                Err(e) => log::error!("Error saving preferences: {}", e),
            }
        });
    }

    pub fn view_presets_mut(&mut self) -> &mut WithPresets<ViewPreferences> {
        let view_prefs_set = self.latest_view_prefs_set;
        &mut self[view_prefs_set]
    }
}

#[derive(Serialize, Deserialize, Debug, Default, Clone, PartialEq, Eq)]
pub struct PuzzleKeybindSets {
    pub active: String,
    pub sets: Vec<Preset<KeybindSet<PuzzleCommand>>>,
}
impl PuzzleKeybindSets {
    pub fn get(&self, set_name: &str) -> Option<&Preset<KeybindSet<PuzzleCommand>>> {
        self.sets.iter().find(|p| p.name == set_name)
    }
    pub fn get_mut(&mut self, set_name: &str) -> &mut Preset<KeybindSet<PuzzleCommand>> {
        match self.sets.iter_mut().find_position(|p| p.name == set_name) {
            Some((i, _)) => &mut self.sets[i],
            None => {
                self.sets.push(Preset {
                    name: set_name.to_string(),
                    value: KeybindSet::default(),
                });
                self.sets.last_mut().unwrap()
            }
        }
    }
    pub fn get_active(&self) -> Vec<&Preset<KeybindSet<PuzzleCommand>>> {
        let mut included_names = vec![&self.active];
        let mut unprocessed_idx = 0;
        while unprocessed_idx < included_names.len() {
            if let Some(set) = self.get(included_names[unprocessed_idx]) {
                for name in &set.value.includes {
                    if !included_names.contains(&name) {
                        included_names.push(name);
                    }
                }
            }
            unprocessed_idx += 1;
        }

        // Standardize order.
        self.sets
            .iter()
            .filter(|set| included_names.contains(&&set.name))
            .collect()
    }
    pub fn get_active_keybinds(&self) -> impl '_ + Iterator<Item = &'_ Keybind<PuzzleCommand>> {
        self.get_active()
            .into_iter()
            .flat_map(|set| &set.value.keybinds)
    }
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
#[serde(default)]
pub struct WithPresets<T: Default> {
    /// Current values (might not be saved).
    ///
    /// If this is `None`, then it is assumed to be taken from the `presets`
    /// list.
    #[serde(skip)]
    pub current: Option<T>,
    /// Name of the most recently-loaded preset.
    pub last_loaded: String,
    /// List of all saved presets.
    pub presets: Vec<Preset<T>>,

    /// Rename operation completed during the last frame, if any.
    #[serde(skip)]
    pub recent_rename_op: Option<(String, String)>,
}
impl<T: Default + Clone> WithPresets<T> {
    pub fn has(&self, name: &str) -> bool {
        self.get(name).is_some()
    }
    pub fn get(&self, name: &str) -> Option<&Preset<T>> {
        self.presets.iter().find(|p| p.name == name)
    }
    pub fn get_mut(&mut self, name: &str) -> Option<&mut Preset<T>> {
        self.presets.iter_mut().find(|p| p.name == name)
    }
    pub fn last_loaded_preset(&self) -> Option<&Preset<T>> {
        self.get(&self.last_loaded.clone())
    }
    pub fn last_loaded_preset_mut(&mut self) -> Option<&mut Preset<T>> {
        self.get_mut(&self.last_loaded.clone())
    }
    pub fn save_preset(&mut self) {
        if let Some(new_value) = self.current.clone() {
            match self.presets.iter_mut().find(|p| p.name == self.last_loaded) {
                Some(p) => p.value = new_value,
                None => {
                    if let Some(p) = self.current_preset() {
                        self.presets.push(p);
                    }
                }
            }
        }
    }
    pub fn add_preset(&mut self, name: String) {
        if let Some(value) = self.current.clone() {
            self.presets.push(Preset { name, value });
        }
    }
    pub fn rename(&mut self, old_name: &str, new_name: &str) {
        if let Some(preset) = self.get_mut(old_name) {
            preset.name = new_name.to_string();
        }
        if self.last_loaded == old_name {
            self.last_loaded = new_name.to_string();
        }
        if self.recent_rename_op.is_some() {
            log::warn!("shadowing unhandled preset rename operation!")
        }
        self.recent_rename_op = Some((old_name.to_string(), new_name.to_string()));
    }
    pub fn delete(&mut self, name: &str) {
        self.presets.retain(|p| p.name != name);
    }
    /// Returns the current preset (what would be saved if the user saves it).
    pub fn current_preset(&self) -> Option<Preset<T>> {
        match &self.current {
            Some(value) => Some(Preset {
                name: self.last_loaded.clone(),
                value: value.clone(),
            }),
            None => self.last_loaded_preset().cloned(),
        }
    }
    /// Sets the current preset name and value.
    pub fn set_current_preset(&mut self, preset: Preset<T>) {
        self.current = Some(preset.value);
        self.last_loaded = preset.name;
    }
    /// Loads a named preset. If there is no preset with the given name, then
    /// this method does nothing.
    pub fn load_preset(&mut self, name: &str) {
        if let Some(p) = self.get(name) {
            self.set_current_preset(p.clone());
        }
    }
    /// Moves the preset `from` to `to`, shifting all the presents in between.
    pub fn reorder(&mut self, from: &str, to: &str) {
        let Some(i) = self.presets.iter().position(|p| p.name == from) else {
            return;
        };
        let Some(j) = self.presets.iter().position(|p| p.name == to) else {
            return;
        };
        if i < j {
            self.presets[i..=j].rotate_left(1);
        } else if j < i {
            self.presets[j..=i].rotate_right(1);
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(default)]
pub struct Preset<T> {
    #[serde(rename = "preset_name")]
    pub name: String,
    #[serde(flatten)]
    pub value: T,
}
impl<T: Default> Default for Preset<T> {
    fn default() -> Self {
        Self {
            name: "unnamed".to_string(),
            value: T::default(),
        }
    }
}

fn is_false(x: &bool) -> bool {
    !x
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
#[serde(default)]
pub struct PieceFilter {
    /// Hexadecimal-encoded bitstring of which pieces are visible.
    #[serde(with = "crate::serde_impl::hex_bitvec")]
    pub visible_pieces: BitVec,
    /// Opacity of hidden pieces.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hidden_opacity: Option<f32>,
}

#[derive(
    Serialize, Deserialize, Debug, Default, Display, EnumIter, Copy, Clone, PartialEq, Eq, Hash,
)]
pub enum PuzzleViewPreferencesSet {
    #[serde(rename = "3D")]
    #[strum(serialize = "3D")]
    Dim3D,
    #[default]
    #[serde(rename = "4D+")]
    #[strum(serialize = "4D+")]
    Dim4D,
}
impl PuzzleViewPreferencesSet {
    pub fn from_ndim(ndim: u8) -> Self {
        match ndim {
            ..=3 => Self::Dim3D,
            4.. => Self::Dim4D,
        }
    }
}
