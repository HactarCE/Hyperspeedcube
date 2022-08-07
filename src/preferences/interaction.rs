use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
#[serde(default)]
pub struct InteractionPreferences {
    pub unhide_grip: bool,

    pub confirm_discard_only_when_scrambled: bool,

    pub dynamic_twist_speed: bool,
    pub twist_duration: f32,
    pub other_anim_duration: f32,

    pub drag_sensitivity: f32,
}
