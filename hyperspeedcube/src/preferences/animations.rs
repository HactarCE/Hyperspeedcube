use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Default, Clone, PartialEq)]
#[serde(default)]
pub struct AnimationPreferences {
    pub dynamic_twist_speed: bool,
    pub twist_duration: f32,
    pub blocking_anim_duration: f32,
}
