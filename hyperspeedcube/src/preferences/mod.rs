//! User preferences.
//!
//! For a list of key names, see `VirtualKeyCode` in this file:
//! https://github.com/rust-windowing/winit/blob/master/src/event.rs

use std::collections::{HashSet, VecDeque};
use std::ops::{Index, IndexMut};
use std::path::PathBuf;
use std::sync::mpsc;

use bitvec::vec::BitVec;
use eyre::{eyre, OptionExt};
use hyperpuzzle::Rgb;
use instant::Duration;
use itertools::Itertools;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

mod gfx;
mod info;
mod interaction;
mod keybinds;
// mod migration; // TODO
mod migration {
    pub const LATEST_VERSION: u32 = 2;
}
mod colors;
mod mousebinds;
#[cfg(not(target_arch = "wasm32"))]
mod persist_local;
#[cfg(target_arch = "wasm32")]
mod persist_web;
mod styles;
mod view;

pub use colors::*;
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
use crate::util::BeforeOrAfter;

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

    pub gfx: GfxPreferences,
    pub interaction: WithPresets<InteractionPreferences>,
    pub styles: StylePreferences,

    pub view_3d: WithPresets<ViewPreferences>,
    pub view_4d: WithPresets<ViewPreferences>,
    #[serde(skip)]
    pub latest_view_prefs_set: PuzzleViewPreferencesSet,

    pub color_palette: GlobalColorPalette,
    pub color_schemes: ColorPreferences,

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
            info,
            gfx,
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
        gfx.post_init();
        interaction.post_init(Some(&DEFAULT_PREFS.interaction));
        styles.post_init();
        view_3d.post_init(Some(&DEFAULT_PREFS.view_3d));
        view_4d.post_init(Some(&DEFAULT_PREFS.view_4d));
        color_palette.post_init();
        color_schemes.post_init();

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

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
#[serde(default)]
pub struct WithPresets<T: Default> {
    /// Current values (might not be saved).
    ///
    /// If this is `None`, then it is assumed to be taken from the `presets`
    /// list.
    #[serde(skip)]
    pub current: T,
    /// Name of the most recently-loaded preset.
    last_loaded: String,
    /// List of all built-in presets.
    #[serde(skip)]
    builtin: Vec<Preset<T>>,
    /// List of all saved user presets.
    #[serde(rename = "presets")]
    user: Vec<Preset<T>>,

    /// Rename operations completed during the last frame, if any.
    #[serde(skip)]
    recent_renames: VecDeque<Rename>,
}
impl<T: Default + Clone + PartialEq> WithPresets<T> {
    /// Sets the builtin presets list.
    ///
    /// Deletes any user presets with the same name.
    pub fn set_builtin_presets(&mut self, builtin_presets: Vec<Preset<T>>) {
        // Remove user presets with name overlap.
        let taken_names: HashSet<&String> = builtin_presets.iter().map(|p| &p.name).collect();
        self.user.retain(|p| !taken_names.contains(&p.name));
        self.builtin = builtin_presets;

        // If the most-recently-loaded preset is a built-in preset, then the
        // color couldn't be loaded when preferences were first deserialized.
        // This solution isn't perfect, but it's good enough.
        if self.current == T::default() && self.builtin.iter().any(|p| p.name == self.last_loaded) {
            self.post_init(None);
        }
    }

    /// Returns the number of presets, including built-in and user presets.
    pub fn len(&self) -> usize {
        self.builtin.len() + self.user.len()
    }
    /// Returns whether there is a preset with the given name.
    pub fn has(&self, name: &str) -> bool {
        self.get(name).is_some()
    }
    /// Returns the preset with the given name, or `None` if it does not exist.
    pub fn get(&self, name: &str) -> Option<&Preset<T>> {
        None.or_else(|| self.user.iter().find(|p| p.name == name))
            .or_else(|| self.builtin.iter().find(|p| p.name == name))
    }
    /// Returns the preset with the given name, or `None` if it does not exist
    /// or cannot be modified.
    fn get_mut(&mut self, name: &str) -> Option<&mut Preset<T>> {
        self.user.iter_mut().find(|p| p.name == name)
    }

    /// Returns the list of built-in presets.
    pub fn builtin_list(&self) -> &[Preset<T>] {
        &self.builtin
    }
    /// Returns the list of saved user presets.
    pub fn user_list(&self) -> &[Preset<T>] {
        &self.user
    }

    /// Returns the name of the most-recently-loaded preset.
    pub fn last_loaded_name(&self) -> &String {
        &self.last_loaded
    }
    /// Returns the most-recently-loaded preset.
    pub fn last_loaded_preset(&self) -> Option<&Preset<T>> {
        self.get(&self.last_loaded)
    }
    /// Returns the current preset, with the name of the most-recently-loaded
    /// preset.
    pub fn current_preset(&self) -> Preset<T> {
        Preset {
            name: self.last_loaded.clone(),
            value: self.current.clone(),
        }
    }
    /// Returns the current preset, with the name that would be saved if the
    /// user saves it. This may be different from the most-recently-loaded
    /// preset.
    pub fn preset_to_save(&self) -> Preset<T> {
        let mut ret = self.current_preset();

        if self.is_modified() {
            while self.builtin.iter().any(|p| p.name == ret.name) {
                // Keep looping until we get a free name, just in case the
                // built-in presets are named pathologically.
                ret.name += " (modified)";
            }
        }

        ret
    }
    /// Sets the current preset name and value.
    pub fn set_current_preset(&mut self, preset: Preset<T>) {
        self.current = preset.value;
        self.last_loaded = preset.name;
    }
    /// Loads a named preset. If there is no preset with the given name, then
    /// this method does nothing.
    pub fn load_preset(&mut self, name: &str) {
        if let Some(p) = self.get(name) {
            self.set_current_preset(p.clone());
        }
    }
    /// Loads the most-recently-loaded preset.
    fn post_init(&mut self, backup: Option<&WithPresets<T>>) {
        self.current = None
            .or_else(|| self.last_loaded_preset())
            .or_else(|| backup?.last_loaded_preset())
            .map(|p| p.value.clone())
            .unwrap_or_default()
    }

    /// Returns whether the current preset has been modified from what was most
    /// recently loaded.
    pub fn is_modified(&self) -> bool {
        Some(&self.current) != self.last_loaded_preset().map(|p| &p.value)
    }
    /// Returns whether the given name is a valid name for a new preset.
    pub fn is_name_available(&self, new_name: &str) -> bool {
        !new_name.is_empty() && !self.has(new_name)
    }

    /// Saves the current settings to the most-recently-loaded preset.
    pub fn save_preset(&mut self) {
        let to_save = self.preset_to_save();
        self.last_loaded = to_save.name.clone();
        match self.user.iter_mut().find(|p| p.name == to_save.name) {
            Some(p) => p.value = to_save.value,
            None => self.user.push(to_save),
        }
    }
    /// Adds a new preset with the current settings. The name is assumed to be
    /// available.
    pub fn add_preset(&mut self, name: String) {
        let value = self.current.clone();
        self.last_loaded = name.clone();
        self.user.push(Preset { name, value });
    }
    /// Renames the preset `old_name` to `new_name`. The new name is assumed to
    /// be available.
    pub fn rename(&mut self, old_name: &str, new_name: &str) {
        if let Some(preset) = self.get_mut(old_name) {
            preset.name = new_name.to_string();
        }
        if self.last_loaded == old_name {
            self.last_loaded = new_name.to_string();
        }
        self.recent_renames.push_back(Rename {
            old: old_name.to_string(),
            new: new_name.to_string(),
        });
    }
    /// Deletes a preset, if it exists.
    pub fn delete(&mut self, name: &str) {
        self.user.retain(|p| p.name != name);
    }
    /// Moves the preset `from` to `to`, shifting all the presents in between.
    pub fn reorder(&mut self, from: &str, to: &str, before_or_after: BeforeOrAfter) {
        let Some(from) = self.user.iter().position(|p| p.name == from) else {
            return;
        };
        let Some(to) = self.user.iter().position(|p| p.name == to) else {
            return;
        };
        crate::util::reorder_list(&mut self.user, from, to, before_or_after);
    }

    /// Returns an iterator over the rename operations that have happened since
    /// the last time this method was called.
    pub fn take_renames(&mut self) -> impl Iterator<Item = Rename> {
        std::mem::take(&mut self.recent_renames).into_iter()
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

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Rename {
    pub old: String,
    pub new: String,
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
