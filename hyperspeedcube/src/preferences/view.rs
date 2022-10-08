use std::ops::RangeInclusive;

use ndpuzzle::math::*;
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

    #[serde(alias = "face_spacing")]
    pub facet_spacing: f32,
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

            facet_spacing: 0.0,
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
    pub const SCALE_RANGE: RangeInclusive<f32> = 0.1..=5.0;
    pub fn view_angle(&self) -> Rotor {
        const X: u8 = 0;
        const Y: u8 = 1;
        const Z: u8 = 2;

        Rotor::from_angle_in_axis_plane(X, Z, self.yaw.to_radians())
            * Rotor::from_angle_in_axis_plane(Z, Y, self.pitch.to_radians())
            * Rotor::from_angle_in_axis_plane(Y, X, self.roll.to_radians())
    }

    // TODO: make a proc macro crate to generate a trait impl like this
    pub fn interpolate(&self, rhs: &Self, t: f32) -> Self {
        Self {
            // I know, I know, I should use rotors for interpolation. But I
            // don't have an easy to way to get euler angles from a rotor and
            // this is fine.
            pitch: util::mix(self.pitch, rhs.pitch, t),
            yaw: util::mix(self.yaw, rhs.yaw, t),
            roll: util::mix(self.roll, rhs.roll, t),

            scale: util::mix(self.scale, rhs.scale, t),
            fov_3d: util::mix(self.fov_3d, rhs.fov_3d, t),
            fov_4d: util::mix(self.fov_4d, rhs.fov_4d, t),
            align_h: util::mix(self.align_h, rhs.align_h, t),
            align_v: util::mix(self.align_v, rhs.align_v, t),
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
            facet_spacing: util::mix(self.facet_spacing, rhs.facet_spacing, t),
            sticker_spacing: util::mix(self.sticker_spacing, rhs.sticker_spacing, t),
            outline_thickness: util::mix(self.outline_thickness, rhs.outline_thickness, t),
            light_ambient: util::mix(self.light_ambient, rhs.light_ambient, t),
            light_directional: util::mix(self.light_directional, rhs.light_directional, t),
            light_pitch: util::mix(self.light_pitch, rhs.light_pitch, t),
            light_yaw: util::mix(self.light_yaw, rhs.light_yaw, t),
        }
    }
}
