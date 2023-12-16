//! User preferences.
//!
//! For a list of key names, see `VirtualKeyCode` in this file:
//! https://github.com/rust-windowing/winit/blob/master/src/event.rs

use bitvec::vec::BitVec;
use hyperpuzzle::Puzzle;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

mod colors;
mod gfx;
mod info;
mod interaction;
mod keybinds;
// mod migration; // TODO
mod migration {
    pub const LATEST_VERSION: u32 = 2;
}
mod mousebinds;
mod opacity;
mod outlines;
#[cfg(not(target_arch = "wasm32"))]
mod persist_local;
#[cfg(target_arch = "wasm32")]
mod persist_web;
mod view;

use crate::commands::{Command, PuzzleCommand, PuzzleMouseCommand};
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

    pub info: InfoPreferences,

    pub gfx: GfxPreferences,
    pub interaction: InteractionPreferences,
    pub opacity: OpacityPreferences,
    pub outlines: OutlinePreferences,

    pub view_3d: WithPresets<ViewPreferences>,
    pub view_4d: WithPresets<ViewPreferences>,

    pub colors: ColorPreferences,

    pub piece_filters: (), // TODO

    pub global_keybinds: Vec<Keybind<Command>>,
    pub puzzle_keybinds: (), // TODO
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
        if self.needs_save {
            self.needs_save = false;

            // Set version number.
            self.version = migration::LATEST_VERSION;

            let result = persist::save(self);

            match result {
                Ok(()) => log::debug!("Saved preferences"),
                Err(e) => log::error!("Error saving preferences: {}", e),
            }
        }
    }

    pub fn view(&self, ty: &Puzzle) -> &ViewPreferences {
        match ty.ndim() {
            ..=3 => &self.view_3d.current,
            4.. => &self.view_4d.current,
        }
    }
    pub fn view_mut(&mut self, ty: &Puzzle) -> &mut ViewPreferences {
        &mut self.view_presets(ty).current
    }

    pub fn view_presets(&mut self, ty: &Puzzle) -> &mut WithPresets<ViewPreferences> {
        match ty.ndim() {
            ..=3 => &mut self.view_3d,
            4.. => &mut self.view_4d,
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
