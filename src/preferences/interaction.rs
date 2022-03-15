use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
#[serde(default)]
pub struct InteractionPreferences {
    pub twist_duration: f32,
    pub dynamic_twist_speed: bool,
}
