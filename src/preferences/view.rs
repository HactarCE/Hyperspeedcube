use serde::{Deserialize, Serialize};
use std::ops::{Deref, DerefMut};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(default)]
pub struct ViewPreferences4D {
    #[serde(flatten)]
    base: ViewPreferences,

    /// 4D FOV, in degrees.
    pub fov_4d: f32,
}
impl Deref for ViewPreferences4D {
    type Target = ViewPreferences;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}
impl DerefMut for ViewPreferences4D {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}
impl Default for ViewPreferences4D {
    fn default() -> Self {
        Self {
            base: Default::default(),
            fov_4d: 30.0,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(default)]
pub struct ViewPreferences3D {
    #[serde(flatten)]
    base: ViewPreferences,

    pub show_frontfaces: bool,
    pub show_backfaces: bool,
}
impl Deref for ViewPreferences3D {
    type Target = ViewPreferences;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}
impl DerefMut for ViewPreferences3D {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}
impl Default for ViewPreferences3D {
    fn default() -> Self {
        Self {
            base: Default::default(),
            show_frontfaces: true,
            show_backfaces: true,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(default)]
pub struct ViewPreferences {
    /// Puzzle angle around Y axis, in degrees.
    pub pitch: f32,
    /// Puzzle angle around X axis, in degrees.
    pub yaw: f32,

    /// Global puzzle scale.
    pub scale: f32,
    /// 3D FOV, in degrees (may be negative).
    pub fov_3d: f32,

    pub face_spacing: f32,
    pub sticker_spacing: f32,

    pub outline_thickness: f32,

    pub light_ambient: f32,
    pub light_directional: f32,
    pub light_pitch: f32,
    pub light_yaw: f32,
}
impl Default for ViewPreferences {
    fn default() -> Self {
        Self {
            pitch: 0_f32,
            yaw: 0_f32,

            scale: 1.0,
            fov_3d: 30_f32,

            face_spacing: 0.0,
            sticker_spacing: 0.0,

            outline_thickness: 1.0,

            light_ambient: 1.0,
            light_directional: 0.0,
            light_pitch: 0.0,
            light_yaw: 0.0,
        }
    }
}
