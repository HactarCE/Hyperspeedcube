use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::hash::Hash;
use std::sync::Arc;

use crate::commands::{Command, PuzzleCommand};
use crate::preferences::{Keybind, Preferences};
use crate::puzzle::*;

pub(super) trait KeybindSetAccessor: 'static + Clone + Hash + Send + Sync {
    type Command: Default + Clone + Eq + Serialize + for<'a> Deserialize<'a>;

    const USE_VK_BY_DEFAULT: bool;

    fn display_name(&self) -> String;

    fn get<'a>(&self, prefs: &'a Preferences) -> &'a [Keybind<Self::Command>];
    fn get_mut<'a>(&self, prefs: &'a mut Preferences) -> &'a mut Vec<Keybind<Self::Command>>;
    fn get_defaults<'a>(&self) -> &'static [Keybind<Self::Command>] {
        self.get(&crate::preferences::DEFAULT_PREFS)
    }

    fn confirm_reset(&self) -> bool {
        let name = self.display_name();
        rfd::MessageDialog::new()
            .set_title(&format!("Reset {name} keybinds",))
            .set_description(&format!("Restore {name} keybinds to defaults?"))
            .set_buttons(rfd::MessageButtons::YesNo)
            .show()
    }

    fn includes_mut<'a>(
        &self,
        _prefs: &'a mut Preferences,
    ) -> Option<(Vec<String>, &'a mut BTreeSet<String>)> {
        None
    }
}

#[derive(Debug, Clone, Hash)]
pub(super) struct PuzzleKeybindsAccessor {
    pub(super) puzzle_type: Arc<PuzzleType>,
    pub(super) set_name: String,
}
impl KeybindSetAccessor for PuzzleKeybindsAccessor {
    type Command = PuzzleCommand;

    const USE_VK_BY_DEFAULT: bool = false; // Position is more important for puzzle keybinds

    fn display_name(&self) -> String {
        format!("{} - {}", self.puzzle_type.family_name, self.set_name)
    }

    fn get<'a>(&self, prefs: &'a Preferences) -> &'a [Keybind<PuzzleCommand>] {
        prefs.puzzle_keybinds[&self.puzzle_type]
            .get(&self.set_name)
            .map(|set| set.value.keybinds.as_slice())
            .unwrap_or(&[])
    }
    fn get_mut<'a>(&self, prefs: &'a mut Preferences) -> &'a mut Vec<Keybind<PuzzleCommand>> {
        &mut prefs.puzzle_keybinds[&self.puzzle_type]
            .get_mut(&self.set_name)
            .value
            .keybinds
    }

    fn includes_mut<'a>(
        &self,
        prefs: &'a mut Preferences,
    ) -> Option<(Vec<String>, &'a mut BTreeSet<String>)> {
        Some((
            prefs.puzzle_keybinds[&self.puzzle_type]
                .sets
                .iter()
                .map(|set| set.preset_name.clone())
                .filter(|set_name| set_name != &self.set_name)
                .collect(),
            &mut prefs.puzzle_keybinds[&self.puzzle_type]
                .get_mut(&self.set_name)
                .value
                .includes,
        ))
    }
}

#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, Hash)]
pub(super) struct GlobalKeybindsAccessor;
impl KeybindSetAccessor for GlobalKeybindsAccessor {
    type Command = Command;

    const USE_VK_BY_DEFAULT: bool = true; // Shortcuts like ctrl+Z should move depending on keyboard layout

    fn display_name(&self) -> String {
        "general".to_string()
    }

    fn get<'a>(&self, prefs: &'a Preferences) -> &'a [Keybind<Self::Command>] {
        &prefs.global_keybinds
    }
    fn get_mut<'a>(&self, prefs: &'a mut Preferences) -> &'a mut Vec<Keybind<Self::Command>> {
        &mut prefs.global_keybinds
    }
}
