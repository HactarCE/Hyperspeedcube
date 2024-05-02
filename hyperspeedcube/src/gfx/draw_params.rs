use bitvec::boxed::BitBox;
use hypermath::prelude::*;
use hyperpuzzle::PerPiece;

use crate::puzzle::{Camera, PieceStyleValues};

/// Complete set of values that determines 3D vertex positions.
#[derive(Debug, Default, Clone, PartialEq)]
pub(crate) struct GeometryCacheKey {
    pub pitch: f32,
    pub yaw: f32,
    pub roll: f32,
    pub scale: f32,
    pub fov_3d: f32,
    pub fov_4d: f32,
    pub show_internals: bool,
    pub facet_shrink: f32,
    pub sticker_shrink: f32,
    pub piece_explode: f32,

    pub target_size: [u32; 2],
    pub rot: Option<pga::Motor>,
    pub piece_transforms: PerPiece<Matrix>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct DrawParams {
    pub cam: Camera,

    /// Mouse position in NDC (normalized device coordinates).
    pub mouse_pos: [f32; 2],

    pub background_color: [u8; 3],
    pub internals_color: [u8; 3],
    pub piece_styles: Vec<(PieceStyleValues, BitBox<u64>)>,
    pub piece_transforms: PerPiece<Matrix>,
}
impl DrawParams {
    /// Returns a vector indicating the direction that light is shining from.
    pub fn light_dir(&self) -> cgmath::Vector3<f32> {
        use cgmath::{Deg, Matrix3, Vector3};

        Matrix3::from_angle_y(Deg(self.cam.prefs.light_yaw))
            * Matrix3::from_angle_x(Deg(-self.cam.prefs.light_pitch)) // pitch>0 means light comes from above
            * Vector3::unit_z()
    }

    pub(super) fn geometry_cache_key(&self, ndim: u8) -> GeometryCacheKey {
        let prefs = &self.cam.prefs;

        GeometryCacheKey {
            pitch: prefs.pitch,
            yaw: prefs.yaw,
            roll: prefs.roll,
            scale: prefs.scale,
            fov_3d: prefs.fov_3d,
            fov_4d: prefs.fov_4d,
            show_internals: self.show_internals(ndim),
            facet_shrink: self.facet_shrink(ndim),
            sticker_shrink: prefs.sticker_shrink,
            piece_explode: prefs.piece_explode,

            target_size: self.cam.target_size,
            rot: Some(self.cam.rot.clone()),
            piece_transforms: self.piece_transforms.clone(),
        }
    }

    pub fn show_internals(&self, ndim: u8) -> bool {
        self.cam.prefs.show_internals && ndim == 3
    }
    pub fn facet_shrink(&self, ndim: u8) -> f32 {
        if self.show_internals(ndim) {
            0.0
        } else {
            self.cam.prefs.facet_shrink
        }
    }
    pub fn sticker_shrink(&self, ndim: u8) -> f32 {
        if self.show_internals(ndim) {
            0.0
        } else {
            self.cam.prefs.sticker_shrink
        }
    }
    pub fn outlines_may_use_sticker_color(&self, ndim: u8) -> bool {
        !self.show_internals(ndim)
            && self.cam.prefs.facet_shrink > 0.0
            && (self.cam.prefs.sticker_shrink > 0.0 || self.cam.prefs.piece_explode > 0.0)
    }
}
