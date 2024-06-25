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
            show_backfaces: true,
            show_behind_4d_camera: true,

            show_internals: true,
            gizmo_scale: 1.0,
            facet_shrink: 0.0,
            sticker_shrink: 0.0,
            piece_explode: 0.0,

            light_pitch: 0.0,
            light_yaw: 0.0,
            face_light_intensity: 1.0,
            outline_light_intensity: 0.0,

            downscale_rate: 1,
            downscale_interpolate: true,
        }
    }
}

impl ViewPreferences {
    // TODO: make a proc macro crate to generate a trait impl like this
    pub fn interpolate(&self, rhs: &Self, t: f32) -> Self {
        use hypermath::util::lerp;

        Self {
            view_scale: lerp(self.view_scale, rhs.view_scale, t),
            fov_3d: lerp(self.fov_3d, rhs.fov_3d, t),
            fov_4d: lerp(self.fov_4d, rhs.fov_4d, t),
            show_frontfaces: lerp_discrete(self.show_frontfaces, rhs.show_frontfaces, t),
            show_backfaces: lerp_discrete(self.show_backfaces, rhs.show_backfaces, t),
            show_behind_4d_camera: lerp_discrete(
                self.show_behind_4d_camera,
                rhs.show_behind_4d_camera,
                t,
            ),
            show_internals: self.show_internals && rhs.show_internals,
            gizmo_scale: lerp(self.gizmo_scale, rhs.gizmo_scale, t),
            facet_shrink: lerp(self.facet_shrink, rhs.facet_shrink, t),
            sticker_shrink: lerp(self.sticker_shrink, rhs.sticker_shrink, t),
            piece_explode: lerp(self.piece_explode, rhs.piece_explode, t),
            face_light_intensity: lerp(self.face_light_intensity, rhs.face_light_intensity, t),
            outline_light_intensity: lerp(
                self.outline_light_intensity,
                rhs.outline_light_intensity,
                t,
            ),
            light_pitch: lerp(self.light_pitch, rhs.light_pitch, t),
            light_yaw: lerp(self.light_yaw, rhs.light_yaw, t),
            downscale_rate: lerp(self.downscale_rate as f32, rhs.downscale_rate as f32, t) as u32,
            downscale_interpolate: lerp_discrete(
                self.downscale_interpolate,
                rhs.downscale_interpolate,
                t,
            ),
        }
    }
}

fn lerp_discrete<T>(a: T, b: T, t: f32) -> T {
    if t < 0.5 {
        a
    } else {
        b
    }
}
