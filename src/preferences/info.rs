use serde::{Deserialize, Serialize};

use crate::puzzle::TwistMetric;

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
#[serde(default)]
pub struct InfoPreferences {
    pub metric: TwistMetric,
    #[serde(skip)]
    pub qtm: bool,
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
