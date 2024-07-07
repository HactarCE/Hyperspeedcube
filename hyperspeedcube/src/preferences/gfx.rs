use instant::Duration;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(default)]
pub struct GfxPreferences {
    pub fps_limit: usize,
    pub msaa: bool,
}
impl Default for GfxPreferences {
    fn default() -> Self {
        Self {
            fps_limit: 60,
            msaa: true,
        }
    }
}
impl GfxPreferences {
    pub(super) fn post_init(&mut self) {}

    /// Returns the duration of one frame based on the configured FPS value.
    pub fn frame_duration(&self) -> Duration {
        Duration::from_secs_f64(1.0 / self.fps_limit as f64)
    }

    /// Returns the MSAA sample count.
    pub fn sample_count(&self) -> u32 {
        if self.msaa {
            4
        } else {
            1
        }
    }
}
