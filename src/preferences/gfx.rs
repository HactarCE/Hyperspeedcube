use serde::{Deserialize, Serialize};
use std::{fmt, time::Duration};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(default)]
pub struct GfxPreferences {
    pub fps: u32,
    pub font_size: f32,
    #[serde(skip)]
    pub lock_font_size: bool,

    pub msaa: Msaa,

    pub label_size: f32, // TODO: remove or move this
}
impl Default for GfxPreferences {
    fn default() -> Self {
        Self {
            fps: 60,
            font_size: 17.0,
            lock_font_size: false,

            msaa: Msaa::_8,

            label_size: 24.0,
        }
    }
}
impl GfxPreferences {
    /// Returns the duration of one frame based on the configured FPS value.
    pub fn frame_duration(&self) -> Duration {
        Duration::from_secs_f64(1.0 / self.fps as f64)
    }
}

#[derive(Serialize, Deserialize, Debug, EnumIter, Copy, Clone, PartialEq, Eq)]
pub enum Msaa {
    #[serde(rename = "0")]
    Off = 1,
    #[serde(rename = "2")]
    #[strum(serialize = "2")]
    _2 = 2,
    #[serde(rename = "4")]
    #[strum(serialize = "4")]
    _4 = 4,
    #[serde(other, rename = "8")]
    #[strum(serialize = "8")]
    _8 = 8,
}
impl fmt::Display for Msaa {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Msaa::Off => write!(f, "Off"),
            Msaa::_2 => write!(f, "2x"),
            Msaa::_4 => write!(f, "4x"),
            Msaa::_8 => write!(f, "8x"),
        }
    }
}
