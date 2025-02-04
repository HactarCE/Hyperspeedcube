use hyperpuzzle_core::TwistMetric;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
#[serde(default)]
pub struct InfoPreferences {
    pub metric: TwistMetric,
    #[serde(skip)]
    pub qtm: bool,

    pub keybinds_reference: KeybindsReferencePreferences,

    pub modifier_toggles: bool,
}

#[derive(Serialize, Deserialize, Debug, Default, Copy, Clone)]
#[serde(default)]
pub struct KeybindsReferencePreferences {
    pub function: bool,
    pub navigation: bool,
    pub numpad: bool,

    pub opacity: f32,

    pub max_font_size: f32,
}
