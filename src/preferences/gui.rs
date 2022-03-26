use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Default, Copy, Clone)]
#[serde(default)]
pub struct GuiPreferences {
    pub keybinds_reference: KeybindsReferencePreferences,
}

#[derive(Serialize, Deserialize, Debug, Default, Copy, Clone)]
#[serde(default)]
pub struct KeybindsReferencePreferences {
    pub function: bool,
    pub navigation: bool,
    pub numpad: bool,

    pub opacity: f32,
}
