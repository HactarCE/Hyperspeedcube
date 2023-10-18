//! User preferences.
//!
//! For a list of key names, see `VirtualKeyCode` in this file:
//! https://github.com/rust-windowing/winit/blob/master/src/event.rs

use bitvec::vec::BitVec;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::collections::{btree_map, BTreeMap};
use std::ops::{Index, IndexMut};
use std::path::PathBuf;

mod colors;
mod gfx;
mod info;
mod interaction;
mod keybinds;
mod migration;
mod mousebinds;
mod opacity;
mod outlines;
#[cfg(not(target_arch = "wasm32"))]
mod persist_local;
#[cfg(target_arch = "wasm32")]
mod persist_web;
mod view;

use crate::commands::{Command, PuzzleCommand, PuzzleMouseCommand};
use crate::puzzle::{traits::*, ProjectionType, PuzzleTypeEnum};
pub use colors::*;
pub use gfx::*;
pub use info::*;
pub use interaction::*;
pub use keybinds::*;
pub use mousebinds::*;
pub use opacity::*;
pub use outlines::*;
#[cfg(not(target_arch = "wasm32"))]
use persist_local as persist;
#[cfg(target_arch = "wasm32")]
use persist_web as persist;
pub use view::*;

const PREFS_FILE_FORMAT: config::FileFormat = config::FileFormat::Yaml;
const DEFAULT_PREFS_STR: &str = include_str!("default.yaml");

lazy_static! {
    pub static ref DEFAULT_PREFS: Preferences =
        serde_yaml::from_str(DEFAULT_PREFS_STR).unwrap_or_default();
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
#[serde(default)]
pub struct Preferences {
    #[serde(skip)]
    pub needs_save: bool,

    /// Preferences file format version.
    #[serde(skip_deserializing)]
    pub version: u32,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub log_file: Option<PathBuf>,

    pub show_welcome_at_startup: bool,

    #[cfg(target_arch = "wasm32")]
    pub use_clipboard_fallback: bool,

    pub info: InfoPreferences,

    pub gfx: GfxPreferences,
    pub interaction: InteractionPreferences,
    pub opacity: OpacityPreferences,
    pub outlines: OutlinePreferences,

    pub view_3d: WithPresets<ViewPreferences>,
    pub view_4d: WithPresets<ViewPreferences>,

    pub colors: ColorPreferences,

    pub piece_filters: PerPuzzle<Vec<Preset<PieceFilter>>>,

    pub global_keybinds: Vec<Keybind<Command>>,
    pub puzzle_keybinds: PerPuzzleFamily<PuzzleKeybindSets>,
    pub mousebinds: Vec<Mousebind<PuzzleMouseCommand>>,
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
            .and_then(migration::try_deserialize)
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
        if self.needs_save {
            self.needs_save = false;

            // Clear empty entries.
            self.piece_filters.map.retain(|_k, v| !v.is_empty());

            // Set version number.
            self.version = migration::LATEST_VERSION;

            let result = persist::save(self);

            match result {
                Ok(()) => log::debug!("Saved preferences"),
                Err(e) => log::error!("Error saving preferences: {}", e),
            }
        }
    }

    pub fn view(&self, ty: impl PuzzleType) -> &ViewPreferences {
        match ty.projection_type() {
            ProjectionType::_3D => &self.view_3d.current,
            ProjectionType::_4D => &self.view_4d.current,
        }
    }
    pub fn view_mut(&mut self, ty: impl PuzzleType) -> &mut ViewPreferences {
        &mut self.view_presets(ty).current
    }

    pub fn view_presets(&mut self, ty: impl PuzzleType) -> &mut WithPresets<ViewPreferences> {
        match ty.projection_type() {
            ProjectionType::_3D => &mut self.view_3d,
            ProjectionType::_4D => &mut self.view_4d,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Default, Clone, PartialEq, Eq)]
pub struct PuzzleKeybindSets {
    pub active: String,
    pub sets: Vec<Preset<KeybindSet<PuzzleCommand>>>,
}
impl PuzzleKeybindSets {
    pub fn get(&self, set_name: &str) -> Option<&Preset<KeybindSet<PuzzleCommand>>> {
        self.sets.iter().find(|p| p.preset_name == set_name)
    }
    pub fn get_mut(&mut self, set_name: &str) -> &mut Preset<KeybindSet<PuzzleCommand>> {
        match self
            .sets
            .iter_mut()
            .find_position(|p| p.preset_name == set_name)
        {
            Some((i, _)) => &mut self.sets[i],
            None => {
                self.sets.push(Preset {
                    preset_name: set_name.to_string(),
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
            .filter(|set| included_names.contains(&&set.preset_name))
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
    #[serde(flatten)]
    pub current: T,
    pub active_preset: Option<Preset<T>>,
    pub presets: Vec<Preset<T>>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(default)]
pub struct Preset<T> {
    pub preset_name: String,
    #[serde(flatten)]
    pub value: T,
}
impl<T: Default> Default for Preset<T> {
    fn default() -> Self {
        Self {
            preset_name: "unnamed".to_string(),
            value: T::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
#[serde(transparent)]
pub struct PerPuzzle<T> {
    map: BTreeMap<String, T>,
    #[serde(skip)]
    default: T,
}
impl<T: Default> Index<PuzzleTypeEnum> for PerPuzzle<T> {
    type Output = T;

    fn index(&self, puzzle_type: PuzzleTypeEnum) -> &Self::Output {
        self.get(puzzle_type).unwrap_or(&self.default)
    }
}
impl<T: Default> IndexMut<PuzzleTypeEnum> for PerPuzzle<T> {
    fn index_mut(&mut self, puzzle_type: PuzzleTypeEnum) -> &mut Self::Output {
        self.entry(puzzle_type).or_default()
    }
}
impl<T> PerPuzzle<T> {
    fn entry(&mut self, puzzle_type: PuzzleTypeEnum) -> btree_map::Entry<'_, String, T> {
        self.map.entry(puzzle_type.name().to_owned())
    }
    fn get(&self, puzzle_type: PuzzleTypeEnum) -> Option<&T> {
        self.map.get(puzzle_type.name())
    }
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
#[serde(transparent)]
pub struct PerPuzzleFamily<T> {
    map: BTreeMap<String, T>,
    #[serde(skip)]
    default: T,
}
impl<T: Default> Index<PuzzleTypeEnum> for PerPuzzleFamily<T> {
    type Output = T;

    fn index(&self, puzzle_type: PuzzleTypeEnum) -> &Self::Output {
        self.get(puzzle_type).unwrap_or(&self.default)
    }
}
impl<T: Default> IndexMut<PuzzleTypeEnum> for PerPuzzleFamily<T> {
    fn index_mut(&mut self, puzzle_type: PuzzleTypeEnum) -> &mut Self::Output {
        self.entry(puzzle_type).or_default()
    }
}
impl<T> PerPuzzleFamily<T> {
    fn entry(&mut self, puzzle_type: PuzzleTypeEnum) -> btree_map::Entry<'_, String, T> {
        self.map
            .entry(puzzle_type.family_internal_name().to_owned())
    }
    fn get(&self, puzzle_type: PuzzleTypeEnum) -> Option<&T> {
        self.map.get(puzzle_type.family_internal_name())
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
