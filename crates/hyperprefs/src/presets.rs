//! Presets, lists of presets, and stable references to presets.
//!
//! Throughout the application, we have [`Preset`]s which are values that the
//! user can create, edit, reorder, and delete. Presets are stored in
//! [`PresetsList`]s, and each preset has a name that is unique within its list.
//!
//! ## [`PresetKind`]
//!
//! A [`PresetsList`] contains two kinds of preset: built-in presets and user
//! presets. Built-in presets cannot be directly created, edited, reordered, or
//! deleted by the user, while user presets can be. Built-in presets always
//! appear before user presets.
//!
//! ## [`PresetRef`]
//!
//! Some preferences reference other presets. For example, it's possible to
//! create a keybind that activates a particular [`super::ViewPreferences`] or
//! [`super::FilterPreset`]. In this case, renaming a preset should preserve the
//! reference by making it use the new name. But if we delete a preset, then the
//! reference should retain the old name in case the user recreates the preset.
//! If the user does create a preset with the same name, then we must relink the
//! reference to the new preset.
//!
//! The first goal (preserving a reference when renaming a preset) could be
//! accomplished using a unique immutable ID for each preset, but the second
//! goal (preserving the name) could not. Instead, we use a [`PresetRef`], which
//! is a thin wrapper around an `Arc<Mutex<String>>`. Each [`Preset`] keeps a
//! list of all active [`PresetRef`]s, and when the preset is renamed it updates
//! the name in each [`PresetRef`]. When a preset is deleted, its references
//! become **dead** and are stored in a [`PresetTombstone`], and are **revived**
//! when a new preset is created with the same name or an old preset is renamed
//! to that name.
//!
//! We reuse [`PresetRef`]s whenever possible, and some operations will
//! garbage-collect them if they have no remaining references. In practice, the
//! only case where multiple [`PresetRef`]s will point to the same preset is
//! when an existing preset is renamed in a way that consumes dead references.
//!
//! ## [`PresetData`]
//!
//! But what if we have a [`PresetsList`] _of [`PresetsList`]s?_ Then when we
//! delete an outer preset, we would lose all the dead references to its inner
//! presets. We solve this using a trait [`PresetData`], which indicates data
//! that should be preserved in the [`PresetTombstone`] along with top-level
//! dead references.

use std::borrow::Cow;
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::hash::Hash;
use std::sync::Arc;

use indexmap::IndexMap;
use itertools::Itertools;
use parking_lot::Mutex;

use super::schema;
use crate::ext::reorderable::{BeforeOrAfter, DragAndDropResponse, ReorderableCollection};

pub const MODIFIED_SUFFIX: &str = " (modified)";

/// Data preserved after deleting a preset.
///
/// This stores any [`PresetRef`]s that used to refer to the preset, and
/// sometimes also data from within the preset (which is probably just more
/// [`PresetRef`]s to inner presets).
#[derive(Debug)]
pub struct PresetTombstone<T: PresetData> {
    pub(crate) dead_refs: Vec<PresetRef>,
    pub(crate) value_tombstone: T::Tombstone,
}
impl<T: PresetData> Default for PresetTombstone<T> {
    fn default() -> Self {
        Self {
            dead_refs: Default::default(),
            value_tombstone: Default::default(),
        }
    }
}
impl<T: PresetData> PresetTombstone<T> {
    fn first_ref(&self) -> Option<PresetRef> {
        self.dead_refs.first().cloned()
    }
}

pub trait PresetData: fmt::Debug + PartialEq {
    /// Data to preserve after deleting a preset, and to restore if it
    /// recreated. This is delete when the program exits.
    ///
    /// This is intended to be used to store references to the preset, such as
    /// [`PresetRef`]s.
    type Tombstone: fmt::Debug + Default;

    /// Removes and returns any data that should be stored in a tombstone.
    fn take_tombstone(&mut self) -> Self::Tombstone;
    /// Adds data from a tombstone, reviving any dead references.
    fn revive_tombstone(&mut self, data: Self::Tombstone);
    /// Calls `revive_tombstone()` if `data` is `Some`; otherwise does nothing.
    fn revive_opt_tombstone(&mut self, data: Option<Self::Tombstone>) {
        if let Some(data) = data {
            self.revive_tombstone(data);
        }
    }
    /// Combines two tombstones.
    fn combine_tombstones(tombstone: &mut Self::Tombstone, extra: Self::Tombstone);
    /// Returns whether a tombstone is completely empty, and so can be
    /// discarded.
    fn is_tombstone_empty(tombstone: &Self::Tombstone) -> bool;
}
macro_rules! impl_preset_data_with_empty_tombstone {
    ($type:ty) => {
        impl PresetData for $type {
            type Tombstone = ();

            fn take_tombstone(&mut self) {}
            fn revive_tombstone(&mut self, _data: ()) {}
            fn combine_tombstones(_tombstone: &mut (), _extra: ()) {}
            fn is_tombstone_empty(_tombstone: &()) -> bool {
                true
            }
        }
    };
}
impl_preset_data_with_empty_tombstone!(hyperpuzzle_core::Rgb);
impl_preset_data_with_empty_tombstone!(super::AnimationPreferences);
impl_preset_data_with_empty_tombstone!(super::ColorScheme);
impl_preset_data_with_empty_tombstone!(super::FilterPreset);
impl_preset_data_with_empty_tombstone!(super::FilterSeqPreset);
impl_preset_data_with_empty_tombstone!(super::InteractionPreferences);
impl_preset_data_with_empty_tombstone!(super::PieceStyle);
impl_preset_data_with_empty_tombstone!(super::ViewPreferences);
impl<T: PresetData> PresetData for Preset<T> {
    type Tombstone = PresetTombstone<T>;

    fn take_tombstone(&mut self) -> Self::Tombstone {
        PresetTombstone {
            dead_refs: std::mem::take(self.named_refs.get_mut()),
            value_tombstone: self.value.take_tombstone(),
        }
    }
    fn revive_tombstone(&mut self, data: Self::Tombstone) {
        self.named_refs
            .get_mut()
            .extend(data.dead_refs.into_iter().filter(|o| o.is_used_elsewhere()));
        self.value.revive_tombstone(data.value_tombstone);
    }
    fn combine_tombstones(tombstone: &mut Self::Tombstone, extra: Self::Tombstone) {
        tombstone.dead_refs.extend(
            extra
                .dead_refs
                .into_iter()
                .filter(|o| o.is_used_elsewhere()),
        );
        T::combine_tombstones(&mut tombstone.value_tombstone, extra.value_tombstone);
    }
    fn is_tombstone_empty(tombstone: &Self::Tombstone) -> bool {
        tombstone.dead_refs.is_empty() && T::is_tombstone_empty(&tombstone.value_tombstone)
    }
}
impl<T: PresetData> PresetData for PresetsList<T> {
    type Tombstone = HashMap<String, PresetTombstone<T>>;

    fn take_tombstone(&mut self) -> Self::Tombstone {
        self.remove_all();
        std::mem::take(self.tombstones.get_mut())
    }
    fn revive_tombstone(&mut self, data: Self::Tombstone) {
        for (k, v) in data {
            match self.get_mut(&k) {
                Some(p) => p.revive_tombstone(v),
                None => self.add_tombstone(k, v),
            }
        }
    }
    fn combine_tombstones(tombstone: &mut Self::Tombstone, extra: Self::Tombstone) {
        for (k, v) in extra {
            Preset::combine_tombstones(tombstone.entry(k).or_default(), v);
        }
    }
    fn is_tombstone_empty(tombstone: &Self::Tombstone) -> bool {
        tombstone.values().all(Preset::is_tombstone_empty)
    }
}

/// Rename operation applied to a preset.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Rename {
    /// Old name of the preset.
    pub old: String,
    /// New name of the preset.
    pub new: String,
}

/// Ordered collection of named elements (called "presets") that can be looked
/// up efficiently by name and can be referenced externally independent of their
/// names.
///
/// Some initial presets may be based on immutable "built-in" values.
///
/// **Do not use this type if `T` contains objects that can be referenced.**
/// That requires a custom type, like [`FilterPresetSeqList`] for example.
#[derive(Debug)]
pub struct PresetsList<T: PresetData> {
    /// Name of the most recently-loaded preset, or an empty string if unknown.
    last_loaded: String,
    /// Name of the default preset, if known.
    default: String,
    /// List of all built-in presets.
    builtin: IndexMap<String, T>,
    /// List of all saved user presets.
    pub(super) user: IndexMap<String, Preset<T>>,

    /// Data from deleted presets. They will be revived if there is ever a new
    /// preset with this name.
    tombstones: Mutex<HashMap<String, PresetTombstone<T>>>,
}
impl<T: PresetData> Default for PresetsList<T> {
    fn default() -> Self {
        Self {
            last_loaded: String::new(),
            default: String::new(),
            builtin: IndexMap::new(),
            user: IndexMap::new(),

            tombstones: Mutex::new(HashMap::new()),
        }
    }
}
impl<T: PresetData> PartialEq for PresetsList<T> {
    fn eq(&self, other: &Self) -> bool {
        self.last_loaded == other.last_loaded
            && self.default == other.default
            && self.builtin == other.builtin
            && self.user == other.user
        // ignore tombstones
    }
}
impl<T: PresetData + Eq> Eq for PresetsList<T> {}
impl<T: PresetData + schema::PrefsConvert + Default> schema::PrefsConvert for PresetsList<T>
where
    T::SerdeFormat: Default,
{
    type DeserContext = T::DeserContext;
    type SerdeFormat = schema::current::PresetsList<T::SerdeFormat>;

    fn to_serde(&self) -> Self::SerdeFormat {
        schema::current::PresetsList {
            last_loaded: self.last_loaded.clone(),
            presets: self.to_serde_map(),
        }
    }
    fn reload_from_serde(&mut self, ctx: &Self::DeserContext, value: Self::SerdeFormat) {
        let schema::current::PresetsList {
            last_loaded,
            presets,
        } = value;

        self.last_loaded = last_loaded;
        self.reload_from_serde_map(ctx, presets);
    }
}
impl<T: PresetData + schema::PrefsConvert> PresetsList<T> {
    pub(crate) fn to_serde_map(&self) -> IndexMap<String, T::SerdeFormat> {
        self.user
            .iter()
            .map(|(name, preset)| (name.clone(), preset.value.to_serde()))
            .collect()
    }
    pub(crate) fn from_serde_map(
        ctx: &T::DeserContext,
        map: IndexMap<String, T::SerdeFormat>,
    ) -> Self {
        let mut ret = Self::default();
        ret.reload_from_serde_map(ctx, map);
        ret
    }
    pub(crate) fn reload_from_serde_map(
        &mut self,
        ctx: &T::DeserContext,
        map: IndexMap<String, T::SerdeFormat>,
    ) {
        self.reload_from_presets_map(
            map.into_iter()
                .map(|(name, value)| (name, T::from_serde(ctx, value))),
        );
    }

    /// Returns whether the color system preferences contains the defaults and
    /// so does not need to be saved.
    pub(crate) fn is_default(&self) -> bool {
        let Self {
            last_loaded,
            default,
            builtin: _,
            user: _,
            tombstones: _,
        } = self;

        (last_loaded.is_empty() || last_loaded == default) && self.user_presets_eq_builtin_presets()
    }
}
impl<T: PresetData> PresetsList<T> {
    pub(super) fn reload_from_presets_map(&mut self, map: impl IntoIterator<Item = (String, T)>) {
        // Remove all presets. Dead references are saved.
        self.remove_all();

        // Add presets back. Dead references are restored.
        for (k, v) in map {
            // `T::from_serde()` could be insufficient to track references if
            // things can reference `T` itself.
            self.save_preset(k, v);
        }
    }

    /// Returns whether the list of user presets exactly equals the list of
    /// built-in presets; i.e., the user hasn't modified them.
    fn user_presets_eq_builtin_presets(&self) -> bool {
        itertools::equal(
            self.builtin.iter(),
            self.user.iter().map(|(k, v)| (k, &v.value)),
        )
    }

    /// Sets the built-in presets list and default preset.
    ///
    /// Deletes any user presets with the same name.
    pub fn set_builtin_presets(&mut self, builtin_presets: IndexMap<String, T>, default: String)
    where
        T: Clone,
    {
        if self.builtin == builtin_presets {
            return;
        }

        if self.user_presets_eq_builtin_presets() {
            // Replace all presets.
            self.remove_all();
            for (name, value) in &builtin_presets {
                self.save_preset(name.clone(), value.clone());
            }
        } else {
            // Update or delete unmodified built-in presets.
            let mut to_delete = vec![];
            for (name, preset) in &mut self.user {
                if self.builtin.get(name) == Some(&preset.value) {
                    match builtin_presets.get(name) {
                        Some(new_value) => preset.value = new_value.clone(),
                        None => to_delete.push(name.clone()),
                    }
                }
            }
            for name in to_delete {
                self.remove(&name);
            }
        }
        self.builtin = builtin_presets;
        self.prune_dead_refs();

        self.default = default;
    }

    /// Sets the built-in presets list and default preset from the given default
    /// preferences.
    pub(super) fn set_builtin_presets_from_default_prefs(
        &mut self,
        ctx: &T::DeserContext,
        defaults: &schema::current::PresetsList<T::SerdeFormat>,
    ) where
        T: Clone + schema::PrefsConvert,
        T::SerdeFormat: Default + Clone,
    {
        let builtin_presets = defaults
            .presets
            .iter()
            .map(|(k, v)| (k.clone(), schema::PrefsConvert::from_serde(ctx, v.clone())))
            .collect();

        let default = if defaults.last_loaded.is_empty() {
            defaults.presets.keys().next().cloned().unwrap_or_default()
        } else {
            defaults.last_loaded.clone()
        };

        self.set_builtin_presets(builtin_presets, default);
    }

    /// Returns whether there are no user presets.
    pub fn is_empty(&self) -> bool {
        self.user.is_empty()
    }
    /// Returns the number of user presets.
    pub fn len(&self) -> usize {
        self.user.len()
    }
    /// Returns whether there is a preset with the given name.
    pub fn contains_key(&self, name: &str) -> bool {
        self.user.contains_key(name)
    }
    /// Returns the preset with the given name and marks it as the
    /// most-recently-loaded preset.
    ///
    /// Returns `None` if the preset doesn't exist.
    pub fn load(&mut self, name: &str) -> Option<ModifiedPreset<T>>
    where
        T: Clone,
    {
        let p = self.get(name)?.to_modifiable();
        self.last_loaded = name.to_owned();
        Some(p)
    }
    /// Returns the preset with the given name, or `None` if it does not exist.
    pub fn get(&self, name: &str) -> Option<&Preset<T>> {
        self.user.get(name)
    }
    /// Returns the preset with the given name, or `None` if it does not exist
    /// or cannot be modified.
    pub fn get_mut(&mut self, name: &str) -> Option<&mut Preset<T>> {
        self.user.get_mut(name)
    }
    /// Returns a reference to a preset with the given name, even if one does
    /// not exist.
    pub fn new_ref(&self, name: &str) -> PresetRef {
        if let Some(preset) = self.get(name) {
            preset.new_ref()
        } else {
            let tombstones_ref = self.tombstones.lock();
            if let Some(dead_ref) = tombstones_ref.get(name).and_then(|l| l.first_ref()) {
                dead_ref.clone()
            } else {
                drop(tombstones_ref); // deadlock happens if we don't do this
                let new_ref = PresetRef {
                    name: Arc::new(Mutex::new(name.to_owned())),
                };
                let mut new_tombstone = PresetTombstone::default();
                new_tombstone.dead_refs.push(new_ref.clone());
                self.add_tombstone(name.to_owned(), new_tombstone);
                // Do not prune dead refs, because that would take O(n) time.
                new_ref
            }
        }
    }

    /// Returns `true` if the user preset has been modified or deleted compared
    /// to the built-in preset with the same name. If there is no built-in
    /// preset with the same name, returns whether a preset with `name` exists.
    pub fn is_preset_modified_from_builtin(&self, name: &str) -> bool {
        self.user.get(name).map(|p| &p.value) != self.builtin.get(name)
    }

    /// Returns the index of the user preset with the given name.
    pub fn get_index_of(&self, name: &str) -> Option<usize> {
        self.user.get_index_of(name)
    }

    /// Returns the map of built-in presets.
    pub fn builtin_presets(&self) -> &IndexMap<String, T> {
        &self.builtin
    }
    /// Iterates over saved user presets.
    pub fn user_presets(&self) -> impl Iterator<Item = &Preset<T>> {
        self.user.values()
    }
    /// Iterates over mutable references to saved user presets.
    pub fn user_presets_mut(&mut self) -> impl Iterator<Item = &mut Preset<T>> {
        self.user.values_mut()
    }

    /// Returns the set of names with existing presets.
    pub fn taken_names(&self) -> HashSet<String> {
        self.user.keys().cloned().collect()
    }

    /// Returns the `n`th saved user preset.
    pub fn nth_user_preset(&self, n: usize) -> Option<(&String, &Preset<T>)> {
        self.user.get_index(n)
    }
    /// Returns a mutable referencne to the `n`th saved user preset.
    pub fn nth_user_preset_mut(&mut self, n: usize) -> Option<(&String, &mut Preset<T>)> {
        self.user.get_index_mut(n)
    }

    /// Returns the name of the most-recently-loaded preset.
    pub fn last_loaded_name(&self) -> &String {
        &self.last_loaded
    }
    /// Returns the most-recently-loaded preset, or `None` if it has been
    /// deleted.
    pub fn last_loaded(&self) -> Option<&Preset<T>> {
        self.get(&self.last_loaded)
    }
    /// Sets the most-recently-loaded preset.
    pub fn set_last_loaded(&mut self, name: String) {
        self.last_loaded = name;
    }
    /// Loads the most-recently-loaded preset, or returns some reasonable value
    /// otherwise.
    pub fn load_last_loaded(&self, default_preset_name: &str) -> ModifiedPreset<T>
    where
        T: Default + Clone,
    {
        match self.last_loaded().or_else(|| self.user_presets().next()) {
            Some(p) => p.to_modifiable(),
            None => ModifiedPreset {
                base: self.new_ref(default_preset_name),
                value: T::default(),
            },
        }
    }

    /// Returns whether a preset has been modified.
    ///
    /// Returns `true` if the preset does not exist.
    pub fn is_modified(&self, p: &ModifiedPreset<T>) -> bool
    where
        T: PartialEq,
    {
        match self.get(&p.base.name.lock()) {
            Some(base) => base.value != p.value,
            None => true,
        }
    }
    /// Returns whether the given name is a valid name for a new preset.
    pub fn is_name_available(&self, new_name: &str) -> bool {
        !new_name.is_empty() && !self.contains_key(new_name)
    }

    /// Saves a preset, overwriting an existing one or adding a new one if it
    /// does not already exist.
    pub fn save_preset<'a>(&mut self, name: impl Into<Cow<'a, str>>, value: T) {
        let name = name.into();
        if let Some(preset) = self.user.get_mut(name.as_ref()) {
            preset.value = value;
        } else {
            let name = name.into_owned();
            let mut preset = Preset::new(name.clone(), value);
            preset.revive_opt_tombstone(self.take_preset_tombstone(&name));
            self.user.insert(name.clone(), preset);
        }
    }
    /// Saves a preset, modifying the name if needed to avoid conflicting with
    /// an existing one. Returns the new name.
    #[must_use]
    pub fn save_preset_with_nonconflicting_name(&mut self, desired_name: &str, value: T) -> String {
        let name = self.find_nonconflicting_name(desired_name);
        self.save_preset(&name, value);
        name
    }
    /// Saves a modified preset, creating it anew if the old one has been
    /// deleted.
    pub fn save_over_preset(&mut self, modified_preset: &ModifiedPreset<T>)
    where
        T: Clone,
    {
        self.save_preset(modified_preset.base.name(), modified_preset.value.clone());
    }
    /// Renames the preset `old_name` to `new_name`. This may overwrite an
    /// existing preset.
    pub fn rename<'a>(&mut self, old_name: &str, new_name: impl Into<Cow<'a, str>>) {
        if let Some((index, _old_name, mut preset)) = self.user.swap_remove_full(old_name) {
            let new_name = new_name.into().into_owned();

            // Update external references.
            preset.name = new_name.clone();
            for r in preset.named_refs.get_mut() {
                *r.name.lock() = new_name.clone();
            }
            // Do not prune dead refs, because that would take O(n) time.

            // Revive previously-dead refs for the new name.
            preset.revive_opt_tombstone(self.take_preset_tombstone(&new_name));
            if self.last_loaded == old_name {
                self.last_loaded = new_name.clone();
            }

            // Insert back into the same index.
            let end_index = self.user.len();
            self.user.insert(new_name, preset);
            self.user.swap_indices(index, end_index);
        }
    }
    fn find_nonconflicting_name(&self, desired_name: &str) -> String {
        (1..)
            .map(|i| match i {
                ..=1 => desired_name.to_owned(),
                2.. => format!("{desired_name} {i}"),
            })
            .find(|name| self.is_name_available(name))
            .expect("no name available")
    }
    pub fn make_nonconflicting_funny_name(
        &self,
        mut autonames: impl Iterator<Item = String>,
    ) -> String {
        autonames
            .find(|name| !self.contains_key(name))
            .expect("ran out of autonames!")
    }
    /// Removes all user presets.
    pub fn remove_all(&mut self) {
        for (name, mut preset) in std::mem::take(&mut self.user) {
            self.add_tombstone(name, preset.take_tombstone());
        }
        self.prune_dead_refs();
    }
    /// Removes a user preset, if it exists. Returns the old value.
    pub fn remove(&mut self, name: &str) -> Option<T> {
        self.user.shift_remove(name).map(|mut preset| {
            // Old references are dead.
            self.add_tombstone(name.to_owned(), preset.take_tombstone());
            // Do not prune dead refs, because that would take O(n) time.
            preset.value
        })
    }
    /// Moves the preset `from` to `to`, shifting all the presents in between.
    ///
    /// Fails silently if either `from` or `to` does not exist.
    pub fn reorder_user_preset(&mut self, from: &str, to: &str, before_or_after: BeforeOrAfter) {
        let Some(from) = self.user.get_index_of(from) else {
            return;
        };
        let Some(to) = self.user.get_index_of(to) else {
            return;
        };
        self.user.reorder(DragAndDropResponse {
            payload: from,
            end: to,
            before_or_after: Some(before_or_after),
        });
    }
    /// Moves the user preset at index `from` to index `to`.
    ///
    /// # Panics
    ///
    /// This method panicks if `to` is greater than or equal to number of user
    /// presets.
    pub fn move_index(&mut self, from: usize, to: usize) {
        self.user.move_index(from, to);
    }
    /// Sorts the list of user presets using the function `sort_key`, whose
    /// results are cached.
    ///
    /// If the list was already sorted, then it is sorted in reverse instead.
    pub fn sort_by_key_or_reverse<K: Ord>(
        &mut self,
        mut sort_key: impl FnMut(&String, &Preset<T>) -> K,
    ) {
        let old_order = self.user.keys().cloned().collect_vec();
        self.user.sort_by_cached_key(&mut sort_key);
        if self.user.keys().eq(&old_order) {
            self.user
                .sort_by_cached_key(|k, v| std::cmp::Reverse(sort_key(k, v)));
        }
    }

    /// Adds a tombstone for a preset that was just deleted.
    pub(crate) fn add_tombstone(&self, name: String, data: PresetTombstone<T>) {
        if !Preset::is_tombstone_empty(&data) {
            Preset::combine_tombstones(self.tombstones.lock().entry(name).or_default(), data);
        }
    }
    /// Removes and returns the tombstone for a preset that was previously
    /// deleted. Returns `None` if no such preset existed, or if it did not need
    /// a tombstone.
    fn take_preset_tombstone(&self, name: &str) -> Option<PresetTombstone<T>> {
        self.tombstones.lock().remove(name)
    }
    /// Removes unused dead references. This takes O(n) time, so we only call it
    /// after other operations that also take O(n) time.
    fn prune_dead_refs(&self) {
        self.tombstones.lock().retain(|_, tombstone| {
            tombstone.dead_refs.retain(|o| o.is_used_elsewhere());
            !Preset::is_tombstone_empty(tombstone)
        });
    }
}

/// Named set of values for some set of settings.
#[derive(Debug)]
pub struct Preset<T> {
    name: String,
    pub value: T,
    named_refs: Mutex<Vec<PresetRef>>,
}
impl<T: PartialEq> PartialEq for Preset<T> {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name && self.value == other.value
    }
}
impl<T: Eq> Eq for Preset<T> {}
impl<T> Preset<T> {
    fn new(name: String, value: T) -> Self {
        Self {
            name,
            value,
            named_refs: Mutex::new(vec![]),
        }
    }
    pub fn name(&self) -> &String {
        &self.name
    }
    pub fn new_ref(&self) -> PresetRef {
        let mut named_refs = self.named_refs.lock();
        if named_refs.is_empty() {
            named_refs.push(PresetRef {
                name: Arc::new(Mutex::new(self.name.clone())),
            });
        }
        named_refs[0].clone()
    }
    pub fn to_modifiable(&self) -> ModifiedPreset<T>
    where
        T: Clone,
    {
        ModifiedPreset {
            base: self.new_ref(),
            value: self.value.clone(),
        }
    }
}

/// Reference to a preset.
#[derive(Debug, Clone)]
pub struct PresetRef {
    pub(crate) name: Arc<Mutex<String>>,
}
impl fmt::Display for PresetRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name.lock())
    }
}
impl PartialEq for PresetRef {
    fn eq(&self, other: &Self) -> bool {
        // Compare by pointer first to prevent deadlocks.
        Arc::ptr_eq(&self.name, &other.name) || *self.name.lock() == *other.name.lock()
    }
}
impl<S: AsRef<str>> PartialEq<S> for PresetRef {
    fn eq(&self, other: &S) -> bool {
        *self.name.lock() == other.as_ref()
    }
}
impl Eq for PresetRef {}
impl Hash for PresetRef {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.name.lock().hash(state);
    }
}
impl PresetRef {
    pub fn name(&self) -> String {
        self.name.lock().clone()
    }
    pub(crate) fn is_used_elsewhere(&self) -> bool {
        Arc::strong_count(&self.name) > 1
    }
    pub fn ptr(&self) -> usize {
        Arc::as_ptr(&self.name) as usize
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModifiedPreset<T> {
    pub base: PresetRef,
    pub value: T,
}
impl<T: Default> Default for ModifiedPreset<T> {
    fn default() -> Self {
        Self {
            base: PresetRef {
                name: Arc::new(Mutex::new(String::new())),
            },
            value: Default::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    impl_preset_data_with_empty_tombstone!(i32);

    #[test]
    fn test_preset_renaming() {
        let mut list = PresetsList::default();
        let a1 = list.new_ref("a");
        let _ = list.save_preset("a".to_owned(), 1);
        let b1 = list.new_ref("b");
        let _ = list.save_preset("b".to_owned(), 2);
        let _ = list.save_preset("c".to_owned(), 3);
        let c1 = list.new_ref("c");
        let a2 = list.new_ref("a");
        let b2 = list.new_ref("b");
        let c2 = list.new_ref("c");
        let b3 = list.get("b").unwrap().new_ref();
        let c3 = list.get("c").unwrap().new_ref();
        list.reorder_user_preset("b", "a", BeforeOrAfter::Before);
        assert_eq!(a1, "a");
        assert_eq!(a2, "a");
        assert_eq!(b1, "b");
        assert_eq!(b2, "b");
        assert_eq!(c1, "c");
        assert_eq!(c2, "c");

        list.rename("a", "q");
        assert_eq!(a1, "q");
        assert_eq!(a2, "q");

        let a3 = list.new_ref("q");
        let a4 = list.new_ref("a");

        list.remove("q");
        assert_eq!(a1, "q");
        assert_eq!(a2, "q");
        assert_eq!(a3, "q");

        list.remove("b");
        list.rename("c", "b");
        assert_eq!(b1, "b");
        assert_eq!(b2, "b");
        assert_eq!(b3, "b");
        assert_eq!(c1, "b");
        assert_eq!(c2, "b");
        assert_eq!(c3, "b");
        list.rename("b", "a");
        assert_eq!(b1, "a");
        assert_eq!(b2, "a");
        assert_eq!(b3, "a");
        assert_eq!(c1, "a");
        assert_eq!(c2, "a");
        assert_eq!(c3, "a");
        assert_eq!(a4, "a");

        let _ = list.save_preset("q".to_owned(), 4);
        list.rename("q", "r");
        assert_eq!(a1, "r");
        assert_eq!(a2, "r");
        assert_eq!(a3, "r");
        assert_eq!(a4, "a");
        assert_eq!(b1, "a");
        assert_eq!(b2, "a");
        assert_eq!(b3, "a");
        assert_eq!(c1, "a");
        assert_eq!(c2, "a");
        assert_eq!(c3, "a");
    }

    #[test]
    fn test_nested_preset_renaming() {
        let mut l = PresetsList::default();
        let _ = l.save_preset("A".to_owned(), PresetsList::default());
        let _ = l.save_preset("B".to_owned(), PresetsList::default());
        let a = &mut l.get_mut("A").unwrap().value;
        let aa1 = a.new_ref("a");
        let _ = a.save_preset("a".to_owned(), 1);
        let aa2 = a.new_ref("a");
        let aa3 = a.get("a").unwrap().new_ref();
        l.remove("A");

        let b = &mut l.get_mut("B").unwrap().value;
        let ba1 = b.new_ref("a");
        let _ = b.save_preset("a".to_owned(), 2);
        let ba2 = b.new_ref("a");
        let ba3 = b.get("a").unwrap().new_ref();

        b.rename("a", "b");
        assert_eq!(b.get("b").unwrap().name, "b");
        assert_eq!(aa1, "a");
        assert_eq!(aa2, "a");
        assert_eq!(aa3, "a");
        assert_eq!(ba1, "b");
        assert_eq!(ba2, "b");
        assert_eq!(ba3, "b");

        l.rename("B", "A");

        let a = &mut l.get_mut("A").unwrap().value;
        let _ = a.save_preset("a".to_owned(), 3);
        assert_eq!(aa1, "a");
        assert_eq!(aa2, "a");
        assert_eq!(aa3, "a");
        assert_eq!(ba1, "b");
        assert_eq!(ba2, "b");
        assert_eq!(ba3, "b");
        a.rename("a", "q");
        a.rename("b", "r");
        assert_eq!(aa1, "q");
        assert_eq!(aa2, "q");
        assert_eq!(aa3, "q");
        assert_eq!(ba1, "r");
        assert_eq!(ba2, "r");
        assert_eq!(ba3, "r");

        l.remove("A");

        let mut existing_presets = PresetsList::default();
        let _ = existing_presets.save_preset("q".to_owned(), 4);
        let _ = existing_presets.save_preset("s".to_owned(), 4);

        let _ = l.save_preset("A".to_owned(), existing_presets);
        let a = &mut l.get_mut("A").unwrap().value;

        a.rename("q", "x");
        a.rename("s", "r");
        a.rename("r", "y");

        assert_eq!(aa1, "x");
        assert_eq!(aa2, "x");
        assert_eq!(aa3, "x");
        assert_eq!(ba1, "y");
        assert_eq!(ba2, "y");
        assert_eq!(ba3, "y");
    }
}
