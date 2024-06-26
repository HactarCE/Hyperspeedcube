use hypermath::prelude::*;
use hyperpuzzle::{PerPiece, PieceMask};

use crate::puzzle::{Camera, PieceStyleValues};

/// Complete set of values that determines 3D puzzle vertex positions.
#[derive(Debug, Default, Clone, PartialEq)]
pub(crate) struct PuzzleGeometryCacheKey {
    fov_3d: f32,
    fov_4d: f32,
    show_internals: bool,
    facet_scale: f32,
    gizmo_scale: f32,
    sticker_shrink: f32,
    piece_explode: f32,

    target_size: [u32; 2],
    rot: Option<pga::Motor>,
    // TODO: piece styles in here?
    piece_transforms: PerPiece<Matrix>,
    // TODO(optimization): matrix to apply on GPU to currently-twisting pieces on GPU
    // TODO: colors in here?
}

/// Complete set of values that determines 3D gizmo vertex positions.
#[derive(Debug, Default, Clone, PartialEq)]
pub(crate) struct GizmoGeometryCacheKey {
    fov_3d: f32,
    fov_4d: f32,
    gizmo_scale: f32,

    target_size: [u32; 2],
    rot: Option<pga::Motor>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct DrawParams {
    /// Number of dimensions of the puzzle.
    ///
    /// This should be the same as the rotation matrix in the camera, and the
    /// rotation matrix of every single piece, but we have it here explicitly
    /// just in case.
    pub ndim: u8,
    /// Parameters controlling the camera and lighting.
    pub cam: Camera,

    /// Mouse cursor position in NDC (normalized device coordinates), if it
    /// has some effect on the render.
    ///
    /// This is set to `None` if the cursor position does not affect the render.
    pub cursor_pos: Option<cgmath::Point2<f32>>,
    /// Whether the cursor is currently dragging the view.
    pub is_dragging_view: bool,

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

    pub(super) fn puzzle_geometry_cache_key(&self) -> PuzzleGeometryCacheKey {
        let prefs = &self.cam.prefs();

        PuzzleGeometryCacheKey {
            fov_3d: prefs.fov_3d,
            fov_4d: prefs.fov_4d,
            show_internals: self.show_internals(),
            facet_scale: self.facet_scale(),
            gizmo_scale: self.gizmo_scale(),
            sticker_shrink: prefs.sticker_shrink,
            piece_explode: prefs.piece_explode,

            target_size: self.cam.target_size,
            rot: Some(self.cam.rot.clone()),
            piece_transforms: self.piece_transforms.clone(),
        }
    }
    pub(super) fn gizmo_geometry_cache_key(&self) -> GizmoGeometryCacheKey {
        let prefs = &self.cam.prefs();

        GizmoGeometryCacheKey {
            fov_3d: prefs.fov_3d,
            fov_4d: prefs.fov_4d,
            gizmo_scale: self.gizmo_scale(),

            target_size: self.cam.target_size,
            rot: Some(self.cam.rot.clone()),
        }
    }

    pub fn show_internals(&self) -> bool {
        self.cam.prefs().show_internals && self.ndim == 3
    }
    pub fn gizmo_scale(&self) -> f32 {
        self.cam.prefs().gizmo_scale * self.facet_scale()
    }
    pub fn facet_scale(&self) -> f32 {
        1.0 - self.facet_shrink()
    }
    pub fn facet_shrink(&self) -> f32 {
        if self.show_internals() {
            0.0
        } else {
            self.cam.prefs().facet_shrink
        }
    }
    pub fn sticker_shrink(&self) -> f32 {
        if self.show_internals() {
            0.0
        } else {
            self.cam.prefs().sticker_shrink
        }
    }
    pub fn outlines_may_use_sticker_color(&self) -> bool {
        !self.show_internals()
            && self.cam.prefs().facet_shrink > 0.0
            && (self.cam.prefs().sticker_shrink > 0.0 || self.cam.prefs().piece_explode > 0.0)
    }
}
