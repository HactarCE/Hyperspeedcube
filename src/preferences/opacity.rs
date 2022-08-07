use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
#[serde(default)]
pub struct OpacityPreferences {
    pub base: f32,
    pub ungripped: f32,
    pub hidden: f32,
    pub selected: f32,
}
