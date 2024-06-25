use hypermath::prelude::*;
use hyperpuzzle::{PerPiece, PieceMask};

use crate::puzzle::{Camera, PieceStyleValues};

/// Complete set of values that determines 3D vertex positions.
#[derive(Debug, Default, Clone, PartialEq)]
pub(crate) struct GeometryCacheKey {
    scale: f32,
    fov_3d: f32,
    fov_4d: f32,
    show_internals: bool,
    facet_shrink: f32,
    sticker_shrink: f32,
    piece_explode: f32,

    target_size: [u32; 2],
    rot: Option<pga::Motor>,
    // TODO: piece styles in here?
    piece_transforms: PerPiece<Matrix>,
    // TODO(optimization): matrix to apply on GPU to currently-twisting pieces on GPU
    // TODO: colors in here?
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct DrawParams {
    pub cam: Camera,

    /// Mouse cursor position in NDC (normalized device coordinates).
    pub cursor_pos: [f32; 2],

    pub background_color: [u8; 3],
    pub internals_color: [u8; 3],
    pub sticker_colors: Vec<[u8; 3]>,
    pub piece_styles: Vec<(PieceStyleValues, PieceMask)>,
    pub piece_transforms: PerPiece<Matrix>,
}
impl DrawParams {
    /// Returns a vector indicating the direction that light is shining from.
    pub fn light_dir(&self) -> cgmath::Vector3<f32> {
        use cgmath::{Deg, Matrix3, Vector3};

        Matrix3::from_angle_y(Deg(self.cam.prefs().light_yaw))
            * Matrix3::from_angle_x(Deg(-self.cam.prefs().light_pitch)) // pitch>0 means light comes from above
            * Vector3::unit_z()
    }

    pub(super) fn geometry_cache_key(&self, ndim: u8) -> GeometryCacheKey {
        let prefs = &self.cam.prefs();

        GeometryCacheKey {
            scale: prefs.view_scale,
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
        self.cam.prefs().show_internals && ndim == 3
    }
    pub fn gizmo_scale(&self, ndim: u8) -> f32 {
        self.cam.prefs().gizmo_scale * (1.0 - self.facet_shrink(ndim))
    }
    pub fn facet_shrink(&self, ndim: u8) -> f32 {
        if self.show_internals(ndim) {
            0.0
        } else {
            self.cam.prefs().facet_shrink
        }
    }
    pub fn sticker_shrink(&self, ndim: u8) -> f32 {
        if self.show_internals(ndim) {
            0.0
        } else {
            self.cam.prefs().sticker_shrink
        }
    }
    pub fn outlines_may_use_sticker_color(&self, ndim: u8) -> bool {
        !self.show_internals(ndim)
            && self.cam.prefs().facet_shrink > 0.0
            && (self.cam.prefs().sticker_shrink > 0.0 || self.cam.prefs().piece_explode > 0.0)
    }
}
