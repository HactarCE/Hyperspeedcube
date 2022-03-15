use serde::{Deserialize, Serialize};

use super::DeserializePerPuzzle;
use crate::puzzle::PuzzleType;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(default)]
pub struct ViewPreferences {
    /// Puzzle angle around Y axis, in degrees.
    pub pitch: f32,
    /// Puzzle angle around X axis, in degrees.
    pub yaw: f32,

    pub scale: f32,
    /// 3D FOV, in degrees (may be negative).
    pub fov_3d: f32,
    /// 4D FOV, in degrees.
    pub fov_4d: f32,

    pub face_spacing: f32,
    pub sticker_spacing: f32,

    pub outline_thickness: f32,
}
impl Default for ViewPreferences {
    fn default() -> Self {
        Self {
            pitch: 0_f32,
            yaw: 0_f32,

            scale: 1.0,
            fov_3d: 30_f32,
            fov_4d: 30_f32,

            face_spacing: 0.0,
            sticker_spacing: 0.0,

            outline_thickness: 1.0,
        }
    }
}
impl DeserializePerPuzzle<'_> for ViewPreferences {
    type Proxy = Self;

    fn deserialize_from(value: Self::Proxy, _ty: PuzzleType) -> Self {
        value
    }
}
