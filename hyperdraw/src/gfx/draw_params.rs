use hypermath::prelude::*;
use hyperpuzzle::{PerPiece, PieceMask};

use crate::{Camera, PieceStyleValues};

/// Complete set of values that determines 3D puzzle vertex positions.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct PuzzleGeometryCacheKey {
    fov_3d: f32,
    fov_4d: f32,
    show_frontfaces: bool,
    show_backfaces: bool,
    show_behind_camera: bool,
    show_internals: bool,
    gizmo_scale: f32,
    facet_scale: f32,
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
pub struct GizmoGeometryCacheKey {
    fov_3d: f32,
    fov_4d: f32,
    gizmo_scale: f32,

    target_size: [u32; 2],
    rot: Option<pga::Motor>,
}

/// Parameters controlling how a puzzle is drawn, including its state.
///
/// This does not include static information, such as the initial geometry of
/// all the stickers.
#[derive(Debug, Clone, PartialEq)]
pub struct DrawParams {
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

    /// RGB for internal faces.
    pub internals_color: [u8; 3],
    /// RGB for each sticker color.
    pub sticker_colors: Vec<[u8; 3]>,
    /// Styles for sets of pieces. The piece masks should be disjoint and their
    /// union should be the set of all pieces in the puzzle.
    pub piece_styles: Vec<(PieceStyleValues, PieceMask)>,
    /// N-dimensional transform for each piece.
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

    /// Returns whether there is any animated style.
    pub fn any_animated(&self) -> bool {
        self.piece_styles.iter().any(|(style_values, _piece_set)| {
            style_values.face_color.is_animated() || style_values.outline_color.is_animated()
        })
    }

    pub(super) fn puzzle_geometry_cache_key(&self) -> PuzzleGeometryCacheKey {
        let prefs = &self.cam.prefs();

        PuzzleGeometryCacheKey {
            fov_3d: prefs.fov_3d,
            fov_4d: prefs.fov_4d,
            show_backfaces: prefs.show_backfaces,
            show_behind_camera: prefs.show_behind_4d_camera,
            show_frontfaces: prefs.show_frontfaces,
            show_internals: self.show_internals(),
            gizmo_scale: self.gizmo_scale(),
            facet_scale: self.facet_scale(),
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

    /// Whether internal stickers are visible.
    pub fn show_internals(&self) -> bool {
        self.cam.prefs().show_internals && self.ndim == 3
    }
    /// Linear scale factor for twist gizmos. (0 to infinity)
    pub fn gizmo_scale(&self) -> f32 {
        self.cam.prefs().gizmo_scale * self.facet_scale()
    }
    /// Linear scale factor for facets, calculated from facet shrink. (0 to 1)
    ///
    /// When showing internals, this is always 1.
    pub fn facet_scale(&self) -> f32 {
        1.0 - self.facet_shrink()
    }
    /// Shrink factor for facets. (0 to 1)
    ///
    /// When showing internals, this is always 0.
    pub fn facet_shrink(&self) -> f32 {
        if self.show_internals() {
            0.0
        } else {
            self.cam.prefs().facet_shrink
        }
    }
    /// Linear sticker shrink factor. (0 to 1)
    ///
    /// When showing internals, this is always 0.
    pub fn sticker_shrink(&self) -> f32 {
        if self.show_internals() {
            0.0
        } else {
            self.cam.prefs().sticker_shrink
        }
    }
}
