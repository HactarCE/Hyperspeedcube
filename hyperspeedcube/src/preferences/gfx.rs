use serde::{Deserialize, Serialize};

// TODO: remove if no longer needed

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(default)]
pub struct GfxPreferences {}
impl Default for GfxPreferences {
    fn default() -> Self {
        Self {}
    }
}
impl GfxPreferences {}
