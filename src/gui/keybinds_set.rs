use std::hash::Hash;

use crate::commands::{Command, PuzzleCommand};
use crate::preferences::{Keybind, Preferences};
use crate::puzzle::*;

pub(super) trait KeybindSet: 'static + Copy + Hash + Send + Sync {
    type Command: Default + Clone + Eq;

    const USE_VK_BY_DEFAULT: bool;

    fn display_name(self) -> &'static str;

    fn get(self, prefs: &Preferences) -> &[Keybind<Self::Command>];
    fn get_mut(self, prefs: &mut Preferences) -> &mut Vec<Keybind<Self::Command>>;
    fn get_defaults(self) -> &'static [Keybind<Self::Command>] {
        self.get(&crate::preferences::DEFAULT_PREFS)
    }

    fn confirm_reset(self) -> bool {
        let name = self.display_name();
        rfd::MessageDialog::new()
            .set_title(&format!("Reset {name} keybinds",))
            .set_description(&format!("Restore {name} keybinds to defaults?"))
            .set_buttons(rfd::MessageButtons::YesNo)
            .show()
    }
}

#[derive(Debug, Copy, Clone, Hash)]
pub(super) struct PuzzleKeybinds(pub(super) PuzzleTypeEnum);
impl KeybindSet for PuzzleKeybinds {
    type Command = PuzzleCommand;

    const USE_VK_BY_DEFAULT: bool = false; // Position is more important for puzzle keybinds

    fn display_name(self) -> &'static str {
        self.0.family_display_name()
    }

    fn get(self, prefs: &Preferences) -> &[Keybind<PuzzleCommand>] {
        &prefs.puzzle_keybinds[self.0]
    }
    fn get_mut(self, prefs: &mut Preferences) -> &mut Vec<Keybind<PuzzleCommand>> {
        &mut prefs.puzzle_keybinds[self.0]
    }
}

#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, Hash)]
pub(super) struct GlobalKeybinds;
impl KeybindSet for GlobalKeybinds {
    type Command = Command;

    const USE_VK_BY_DEFAULT: bool = true; // Shortcuts like ctrl+Z should move depending on keyboard layout

    fn display_name(self) -> &'static str {
        "general"
    }

    fn get(self, prefs: &Preferences) -> &[Keybind<Self::Command>] {
        &prefs.global_keybinds
    }
    fn get_mut(self, prefs: &mut Preferences) -> &mut Vec<Keybind<Self::Command>> {
        &mut prefs.global_keybinds
    }
}
