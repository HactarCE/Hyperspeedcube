//! User preferences.
//!
//! For a list of key names, see `VirtualKeyCode` in this file:
//! https://github.com/rust-windowing/winit/blob/master/src/event.rs

use std::ops::{Index, IndexMut};
use std::path::PathBuf;
use std::sync::mpsc;

use bitvec::vec::BitVec;
use eyre::{eyre, OptionExt};
use hyperpuzzle::Rgb;
use itertools::Itertools;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

mod animations;
mod colors;
mod filters;
mod image_generator;
mod info;
mod interaction;
mod keybinds;
mod mousebinds;
#[cfg(not(target_arch = "wasm32"))]
mod persist_local;
#[cfg(target_arch = "wasm32")]
mod persist_web;
mod presets;
mod styles;
mod view;

mod migration {
    pub const LATEST_VERSION: u32 = 2;
}

pub use animations::*;
pub use colors::*;
pub use filters::*;
pub use image_generator::*;
pub use info::*;
pub use interaction::*;
pub use keybinds::*;
pub use mousebinds::*;
#[cfg(not(target_arch = "wasm32"))]
use persist_local as persist;
#[cfg(target_arch = "wasm32")]
use persist_web as persist;
pub use presets::*;
pub use styles::*;
pub use view::*;

use crate::commands::{Command, PuzzleCommand, PuzzleMouseCommand};

const PREFS_FILE_FORMAT: config::FileFormat = config::FileFormat::Yaml;
const DEFAULT_PREFS_STR: &str = include_str!("default.yaml");

lazy_static! {
    pub static ref DEFAULT_PREFS: Preferences =
        serde_yml::from_str(DEFAULT_PREFS_STR).expect("error loading default preferences");
    static ref PREFS_SAVE_THREAD: (
        mpsc::Sender<PrefsSaveCommand>,
        Mutex<Option<std::thread::JoinHandle<()>>>
    ) = spawn_save_thread();
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

    pub info: InfoPreferences,

    pub image_generator: ImageGeneratorPreferences,

    pub animation: WithPresets<AnimationPreferences>,
    pub interaction: WithPresets<InteractionPreferences>,
    pub styles: StylePreferences,

    pub view_3d: WithPresets<ViewPreferences>,
    pub view_4d: WithPresets<ViewPreferences>,
    #[serde(skip)]
    pub latest_view_prefs_set: PuzzleViewPreferencesSet,

    pub color_palette: GlobalColorPalette,
    pub color_schemes: ColorPreferences,

    pub piece_filters: FilterPreferences,

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
            .post_init()
    }

    pub fn save(&mut self) {
        self.needs_save = false;
        self.needs_save_eventually = false;
        let (tx, _join_handle) = &*PREFS_SAVE_THREAD;
        if let Err(e) = tx.send(PrefsSaveCommand::Save(self.clone())) {
            log::error!("Error saving preferences: {e}")
        }
    }
    pub fn block_on_final_save(&mut self) {
        // IIFE to mimic try_block
        let result = (|| -> eyre::Result<()> {
            let (tx, join_handle) = &*PREFS_SAVE_THREAD;
            tx.send(PrefsSaveCommand::Quit)?;
            let join_handle = join_handle
                .lock()
                .take()
                .ok_or_eyre("no thread join handle")?;
            join_handle.join().map_err(|e| eyre!("{e:?}"))?;
            Ok(())
        })();
        if let Err(e) = result {
            log::error!("Error waiting for preferences saving: {e}");
            return;
        }
    }

    pub fn view_presets_mut(&mut self) -> &mut WithPresets<ViewPreferences> {
        let view_prefs_set = self.latest_view_prefs_set;
        &mut self[view_prefs_set]
    }

    /// Modifies the preferences to ensure that any invariants not encoded into
    /// the type are respected.
    ///
    /// For example, this ensures that the default color names are correct. It
    /// also loads each preset.
    #[must_use]
    fn post_init(mut self) -> Self {
        let Self {
            needs_save: _,
            needs_save_eventually: _,
            version,
            log_file: _,
            image_generator: _,
            info,
            animation,
            interaction,
            styles,
            view_3d,
            view_4d,
            latest_view_prefs_set: _,
            color_palette,
            color_schemes,
            piece_filters,
            global_keybinds,
            puzzle_keybinds,
            mousebinds,
        } = &mut self;

        *version = migration::LATEST_VERSION;
        info.post_init();
        animation.post_init(Some(&DEFAULT_PREFS.animation));
        interaction.post_init(Some(&DEFAULT_PREFS.interaction));
        styles.post_init();
        view_3d.post_init(Some(&DEFAULT_PREFS.view_3d));
        view_4d.post_init(Some(&DEFAULT_PREFS.view_4d));
        color_palette.post_init();
        color_schemes.post_init();
        piece_filters.post_init();

        self
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
    Serialize,
    Deserialize,
    Debug,
    Default,
    Display,
    AsRefStr,
    EnumIter,
    Copy,
    Clone,
    PartialEq,
    Eq,
    Hash,
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

fn spawn_save_thread() -> (
    mpsc::Sender<PrefsSaveCommand>,
    Mutex<Option<std::thread::JoinHandle<()>>>,
) {
    let (tx, rx) = mpsc::channel();

    let join_handle = std::thread::spawn(move || {
        for command in rx {
            match command {
                PrefsSaveCommand::Save(prefs) => {
                    let result = persist::save(&prefs);
                    match result {
                        Ok(()) => log::debug!("Saved preferences"),
                        Err(e) => log::error!("Error saving preferences: {e}"),
                    }
                }
                PrefsSaveCommand::Quit => return,
            }
        }
    });

    (tx, Mutex::new(Some(join_handle)))
}

#[derive(Debug)]
enum PrefsSaveCommand {
    Save(Preferences),
    Quit,
}

#[cfg(test)]
mod tests {
    use indexmap::IndexMap;

    #[test]
    fn test_yaml_preserves_map_order() {
        let mut m = IndexMap::<String, usize>::new();
        m.insert("tenpo".to_string(), 0);
        m.insert("mute".to_string(), 1);
        m.insert("la".to_string(), 2);
        m.insert("mi".to_string(), 3);
        m.insert("toki".to_string(), 4);
        m.insert("pona".to_string(), 5);
        let serialized = serde_yml::to_string(&m).unwrap();
        let deserialized: IndexMap<String, usize> = serde_yml::from_str(&serialized).unwrap();
        for (i, &v) in deserialized.values().enumerate() {
            assert_eq!(i, v);
        }
    }
}
