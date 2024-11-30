use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(default)]
pub struct ViewPreferences {
    /// View scale.
    pub view_scale: f32,
    /// 3D FOV, in degrees (may be negative).
    pub fov_3d: f32,
    /// 4D FOV, in degrees.
    pub fov_4d: f32,

    /// Show 3D or 4D frontfaces.
    pub show_frontfaces: bool,
    /// Show 3D or 4D backfaces.
    pub show_backfaces: bool,
    /// Show geometry behind the 4D camera.
    pub show_behind_4d_camera: bool,

    /// Show internal pieces in 3D.
    pub show_internals: bool,
    /// Gizmo scale, on top of facet shrink.
    pub gizmo_scale: f32,
    /// Facet shrink.
    pub facet_shrink: f32,
    /// Sticker shrink.
    pub sticker_shrink: f32,
    /// Piece explode.
    pub piece_explode: f32,

    /// Pitch of 3D directional light source.
    pub light_pitch: f32,
    /// Yaw of 3D directional light source.
    pub light_yaw: f32,
    /// Intensity of directional lighting on faces.
    pub face_light_intensity: f32,
    /// Intensity of directional lighting on outlines.
    pub outline_light_intensity: f32,

    /// Number of pixels in the UI per pixel in the render. This is mainly used
    /// for debugging.
    pub downscale_rate: u32,
    /// Whether to use bilinear sampling instead of nearest-neighbor sampling to
    /// upscale the render.
    pub downscale_interpolate: bool,
}
impl Default for ViewPreferences {
    fn default() -> Self {
        Self {
            view_scale: 1.0,
            fov_3d: 0.0,
            fov_4d: 30.0,

            show_frontfaces: true,
            show_backfaces: false,
            show_behind_4d_camera: false,

            show_internals: false,
            gizmo_scale: 1.0,
            facet_shrink: 0.0,
            sticker_shrink: 0.0,
            piece_explode: 0.0,

            light_pitch: 0.0,
            light_yaw: 0.0,
            face_light_intensity: 0.0,
            outline_light_intensity: 0.0,

            downscale_rate: 1,
            downscale_interpolate: true,
        }
    }
}
