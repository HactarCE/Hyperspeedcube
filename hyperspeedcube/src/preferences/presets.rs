use std::collections::{HashSet, VecDeque};

use serde::{Deserialize, Serialize};

use crate::ext::reorderable::{BeforeOrAfter, DragAndDropResponse, ReorderableCollection};

pub const MODIFIED_SUFFIX: &str = " (modified)";

/// Rename operation applied to a preset.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Rename {
    /// Old name of the preset.
    pub old: String,
    /// New name of the preset.
    pub new: String,
}

/// Set of named presets for a set of settings, along with a current value and
/// the named preset it is based on.
#[derive(Serialize, Deserialize, Debug, Default, Clone)]
#[serde(default)]
pub struct WithPresets<T: Default> {
    /// Current values (not saved).
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
    pub(super) user: Vec<Preset<T>>,

    /// Rename operations completed during the last frame, if any.
    ///
    /// TODO: review these
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
                ret.name += MODIFIED_SUFFIX;
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
    pub(super) fn post_init(&mut self, backup: Option<&WithPresets<T>>) {
        self.current = None
            .or_else(|| self.last_loaded_preset())
            .or_else(|| backup?.last_loaded_preset())
            .map(|p| p.value.clone())
            .unwrap_or_default();
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
    pub fn reorder_user_preset(&mut self, from: &str, to: &str, before_or_after: BeforeOrAfter) {
        let Some(from) = self.user.iter().position(|p| p.name == from) else {
            return;
        };
        let Some(to) = self.user.iter().position(|p| p.name == to) else {
            return;
        };
        self.user.reorder(DragAndDropResponse {
            payload: from,
            end: to,
            before_or_after: Some(before_or_after),
        });
    }

    /// Returns an iterator over the rename operations that have happened since
    /// the last time this method was called.
    pub fn take_renames(&mut self) -> impl Iterator<Item = Rename> {
        std::mem::take(&mut self.recent_renames).into_iter()
    }
}

/// Named set of values for some set of settings.
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
