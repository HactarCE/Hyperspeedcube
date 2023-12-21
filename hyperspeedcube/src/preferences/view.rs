use cgmath::{Deg, Quaternion, Rotation3};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(default)]
pub struct ViewPreferences {
    /// Puzzle angle around X axis, in degrees.
    pub pitch: f32,
    /// Puzzle angle around Y axis, in degrees.
    pub yaw: f32,
    /// Puzzle angle around Z axis, in degrees.
    pub roll: f32,

    /// Global puzzle scale.
    pub scale: f32,
    /// 3D FOV, in degrees (may be negative).
    pub fov_3d: f32,
    /// 4D FOV, in degrees.
    pub fov_4d: f32,

    /// Horizontal alignment, from -1.0 to +1.0.
    pub align_h: f32,
    /// Vertical alignment, from -1.0 to +1.0.
    pub align_v: f32,

    pub show_frontfaces: bool,
    pub show_backfaces: bool,
    pub clip_4d: bool,

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
            roll: 0_f32,

            scale: 1.0,
            fov_3d: 30_f32,
            fov_4d: 30_f32,

            align_h: 0.0,
            align_v: 0.0,

            face_spacing: 0.0,
            sticker_spacing: 0.0,

            show_frontfaces: true,
            show_backfaces: true,
            clip_4d: true,

            outline_thickness: 1.0,

            light_ambient: 1.0,
            light_directional: 0.0,
            light_pitch: 0.0,
            light_yaw: 0.0,
        }
    }
}

impl ViewPreferences {
    pub fn view_angle(&self) -> Quaternion<f32> {
        Quaternion::from_angle_z(Deg(self.roll))
            * Quaternion::from_angle_x(Deg(self.pitch))
            * Quaternion::from_angle_y(Deg(self.yaw))
    }

    // TODO: make a proc macro crate to generate a trait impl like this
    pub fn interpolate(&self, rhs: &Self, t: f32) -> Self {
        use hypermath::util::lerp;

        Self {
            // I know, I know, I should use quaternions for interpolation. But
            // cgmath uses XYZ order by default instead of YXZ so doing this
            // properly isn't trivial.
            pitch: lerp(self.pitch, rhs.pitch, t),
            yaw: lerp(self.yaw, rhs.yaw, t),
            roll: lerp(self.roll, rhs.roll, t),

            scale: lerp(self.scale, rhs.scale, t),
            fov_3d: lerp(self.fov_3d, rhs.fov_3d, t),
            fov_4d: lerp(self.fov_4d, rhs.fov_4d, t),
            align_h: lerp(self.align_h, rhs.align_h, t),
            align_v: lerp(self.align_v, rhs.align_v, t),
            show_frontfaces: if t < 0.5 {
                self.show_frontfaces
            } else {
                rhs.show_frontfaces
            },
            show_backfaces: if t < 0.5 {
                self.show_backfaces
            } else {
                rhs.show_backfaces
            },
            clip_4d: if t < 0.5 { self.clip_4d } else { rhs.clip_4d },
            face_spacing: lerp(self.face_spacing, rhs.face_spacing, t),
            sticker_spacing: lerp(self.sticker_spacing, rhs.sticker_spacing, t),
            outline_thickness: lerp(self.outline_thickness, rhs.outline_thickness, t),
            light_ambient: lerp(self.light_ambient, rhs.light_ambient, t),
            light_directional: lerp(self.light_directional, rhs.light_directional, t),
            light_pitch: lerp(self.light_pitch, rhs.light_pitch, t),
            light_yaw: lerp(self.light_yaw, rhs.light_yaw, t),
        }
    }
}
