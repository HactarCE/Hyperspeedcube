use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Default, Clone, PartialEq)]
#[serde(default)]
pub struct InteractionPreferences {
    pub confirm_discard_only_when_scrambled: bool,

    pub drag_sensitivity: f32,
    // TODO: drag sensitivity for rotations vs. twists
    pub scale_twist_drag_by_radius: bool,
    pub realign_on_release: bool,
    pub realign_on_keypress: bool,
    pub smart_realign: bool,
}
