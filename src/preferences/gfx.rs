use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(default)]
pub struct GfxPreferences {
    pub msaa: bool,
}
impl Default for GfxPreferences {
    fn default() -> Self {
        Self { msaa: true }
    }
}
impl GfxPreferences {
    /// Returns the MSAA sample count.
    pub fn sample_count(&self) -> u32 {
        if self.msaa {
            4
        } else {
            1
        }
    }
}
