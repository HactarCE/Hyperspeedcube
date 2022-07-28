use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
#[serde(default)]
pub struct InteractionPreferences {
    pub confirm_discard_only_when_scrambled: bool,

    pub highlight_piece_on_hover: bool,

    pub selection_fade_duration: f32,
    pub hover_fade_duration: f32,
    pub twist_duration: f32,
    pub dynamic_twist_speed: bool,

    pub drag_sensitivity: f32,
}
