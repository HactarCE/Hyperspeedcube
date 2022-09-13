use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
#[serde(default)]
pub struct OpacityPreferences {
    pub base: f32,
    pub ungripped: f32,
    pub hidden: f32,
    pub selected: f32,

    pub unhide_grip: bool,

    pub save_opacity_in_piece_filter_preset: bool,
}
