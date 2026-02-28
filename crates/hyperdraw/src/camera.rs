use std::time::Instant;

use cgmath::EuclideanSpace;
use eyre::{Result, bail};
use hypermath::{pga::Motor, prelude::*};
use hyperprefs::{AnimationPreferences, ModifiedPreset, ViewPreferences};

/// `w_divisor` below which geometry gets clipped.
const W_DIVISOR_CLIPPING_PLANE: f32 = 0.1;

const DEFAULT_ZOOM: f32 = 0.5;

/// Parameters controlling the camera and lighting.
#[derive(Debug, Clone, PartialEq)]
pub struct NdEuclidCamera {
    /// Number of dimensions for the camera.
    ndim: u8,

    /// Current view settings.
    pub view_preset: ModifiedPreset<ViewPreferences>,

    /// Width and height of the draw target in pixels.
    pub target_size: [u32; 2],

    /// Rotation to apply to the puzzle before drawing it.
    rot: Motor,
    /// Linear factor by which to scale the puzzle before drawing it.
    pub zoom: f32,

    /// Rotation animation, represented as the start & end motors and a start
    /// time.
    rot_animation: Option<([Motor; 2], Instant)>,
}
impl NdEuclidCamera {
    /// Constructs a new default camera.
    pub fn new(ndim: u8, view_preset: ModifiedPreset<ViewPreferences>) -> Self {
        Self {
            ndim,
            view_preset,
            target_size: [1, 1],
            rot: Motor::ident(ndim),
            zoom: DEFAULT_ZOOM,
            rot_animation: None,
        }
    }

    /// Returns the view preferences that the camera is using.
    pub fn prefs(&self) -> &ViewPreferences {
        &self.view_preset.value
    }

    /// Begins a recentering animation from `target_vector` to the top (2D),
    /// front (3D), or the "in" of the 4D projection (4D+).
    ///
    /// - If `reverse` is `true`, then the bottom/back/"out" is used instead.
    /// - If `anti` is `true`, then the rotation delta is reversed.
    pub fn animate_recenter(&mut self, target_vector: impl VectorRef, reverse: bool, anti: bool) {
        let mut initial_vector = self.rot.transform_vector(target_vector);
        let mut final_vector = if self.ndim >= 4 {
            -Vector::unit(3) // -W
        } else {
            Vector::unit(self.ndim - 1)
        };
        if anti {
            final_vector = -final_vector;
        }
        if reverse {
            std::mem::swap(&mut initial_vector, &mut final_vector);
        }
        let rotation_delta = Motor::rotation_infallible(initial_vector, final_vector);
        self.animate_camera_toward(rotation_delta * &self.rot);
    }
    /// Begins animating the camera toward the target rotation.
    fn animate_camera_toward(&mut self, target_rot: Motor) {
        self.rot_animation = Some(([self.rot.clone(), target_rot], Instant::now()));
    }
    /// Cancels an animation if one is in progress.
    fn cancel_animation(&mut self) {
        self.rot_animation = None;
    }

    /// Updates the camera animations.
    pub fn update_animations(&mut self, prefs: &AnimationPreferences) {
        if let Some(([start_rot, end_rot], start_time)) = &self.rot_animation {
            let t = start_time.elapsed().as_secs_f32() / prefs.twist_duration;
            if t > 1.0 {
                self.rot = end_rot.clone();
                self.rot_animation = None;
            } else {
                self.rot = Motor::slerp_infallible(
                    start_rot,
                    end_rot,
                    prefs.twist_interpolation.interpolate(t) as f64,
                );
            }
        }
    }

    /// Resets the camera rotation and zoom.
    pub fn reset(&mut self) {
        self.rot = Motor::ident(self.ndim);
        self.zoom = DEFAULT_ZOOM;
    }

    /// Returns the current camera rotation.
    pub fn rot(&self) -> &Motor {
        &self.rot
    }
    /// Resets the camera rotation.
    pub fn reset_rot(&mut self) {
        self.set_rot(Motor::ident(self.ndim));
    }
    /// Sets the camera rotation and cancels any rotation animation.
    ///
    /// Does nothing if `new_rot` has too many dimensions.
    pub fn set_rot(&mut self, new_rot: Motor) {
        if new_rot.ndim() > self.ndim {
            return;
        }
        self.cancel_animation();
        self.rot = new_rot.to_ndim_at_least(self.ndim);
    }
    /// Applies a delta to the left of the camera rotation.
    ///
    /// Does nothing if `delta` has too many dimensions.
    pub fn rot_by(&mut self, delta: Motor) {
        self.set_rot(delta * &self.rot);
    }

    /// Returns the number of pixels in 1 screen space unit.
    fn compute_pixel_scale(target_size: [u32; 2], zoom: f32) -> Result<f32> {
        let w = target_size[0] as f32;
        let h = target_size[1] as f32;
        let min_dimen = f32::min(w, h);
        if min_dimen == 0.0 {
            bail!("puzzle view has zero size");
        }
        Ok(min_dimen * zoom)
    }
    /// Returns the size of a pixel in screen space.
    fn compute_pixel_size(target_size: [u32; 2], zoom: f32) -> Result<f32> {
        Ok(1.0 / Self::compute_pixel_scale(target_size, zoom)?)
    }

    /// Returns the target size in pixels as a vector.
    pub fn target_size_f32(&self) -> cgmath::Vector2<f32> {
        cgmath::Vector2::from(self.target_size.map(|x| x as f32))
    }
    /// Returns the size of a pixel in screen space.
    pub fn pixel_size(&self) -> Result<f32> {
        Self::compute_pixel_size(self.target_size, self.zoom)
    }
    /// Returns the X and Y scale factors to convert screen space to NDC.
    /// Returns `Err` if either the width or height is smaller than one pixel.
    pub fn xy_scale(&self) -> Result<cgmath::Vector2<f32>> {
        let pixel_scale = Self::compute_pixel_scale(self.target_size, self.zoom)?;
        let w = self.target_size[0] as f32;
        let h = self.target_size[1] as f32;
        Ok(cgmath::vec2(pixel_scale / w, pixel_scale / h))
    }

    /// Returns the global scale of the puzzle.
    pub fn global_scale(&self) -> f32 {
        // Scale the whole puzzle to compensate for facet shrink, and scale back
        // from piece explode.
        1.0 / (1.0 - self.prefs().facet_shrink * 0.5) / (1.0 + self.prefs().piece_explode)
    }

    /// Returns the factor by which the W coordinate affects the XYZ coordinates
    /// during 4D projection.
    pub fn w_factor_4d(&self) -> f32 {
        (self.prefs().fov_4d.to_radians() * 0.5).tan()
    }
    /// Returns the factor by which the Z coordinate affects the XY coordinates
    /// during 3D projection.
    pub fn w_factor_3d(&self) -> f32 {
        (self.prefs().fov_3d.to_radians() * 0.5).tan()
    }
    /// Returns the 4D perspective divisor based on the W coordinate of a point.
    pub fn w_divisor(&self, w: f32) -> f32 {
        // Offset the model along W and keep the new W=0 plane fixed with
        // respect to FOV changes.
        1.0 + (1.0 - w) * self.w_factor_4d()
    }
    /// Returns the 3D perspective divisor based on the Z coordinate of a point.
    pub fn z_divisor(&self, z: f32) -> f32 {
        // Offset the model along Z and keep the new Z=0 plane fixed with
        // respect to FOV changes.
        1.0 + (self.prefs().fov_3d.signum() - z) * self.w_factor_3d()
    }
    /// Projects an N-dimensional point to a 3D point in normalized device
    /// coordinates.
    ///
    /// Be sure to divide by the W coordinate before putting this on the screen.
    fn project_point_to_3d_screen_space(&self, p: &Point) -> Option<cgmath::Vector4<f32>> {
        // This mimics a similar function in the WGSL shader.
        let p = self.rot.transform(p); // Rotate
        let p = hypermath_to_cgmath_vec4(p.as_vector()); // Convert to cgmath vector
        let p = p * self.global_scale(); // Scale

        // Clip geometry that is behind the 4D camera.
        if !self.prefs().show_behind_4d_camera && self.w_divisor(p.w) < W_DIVISOR_CLIPPING_PLANE {
            return None;
        }

        let p = self.project_4d_to_3d(p); // Apply 4D perspective transformation
        let mut p = p.to_homogeneous();
        p.w = self.z_divisor(p.z);
        Some(p)
    }
    /// Projects a 3D point in screen space to normalized device coordinates.
    pub fn project_3d_screen_space_to_ndc(
        &self,
        p: cgmath::Vector4<f32>,
    ) -> Option<cgmath::Point2<f32>> {
        self.scale_screen_space_to_ndc(cgmath::point2(p.x, p.y) / p.w) // Apply scaling
    }
    /// Projects an N-dimensional point to a 2D point in normalized device
    /// coordinates.
    pub fn project_point_to_ndc(&self, p: &Point) -> Option<cgmath::Point2<f32>> {
        let p = self.project_point_to_3d_screen_space(p)?;
        let p = self.project_3d_to_2d(cgmath::Point3::from_homogeneous(p)); // Apply 3D perspective transformation
        self.scale_screen_space_to_ndc(p) // Apply scaling
    }

    fn project_4d_to_3d(&self, p: cgmath::Vector4<f32>) -> cgmath::Point3<f32> {
        // Apply 4D perspective transformation.
        cgmath::Point3::from_vec(p.truncate()) / self.w_divisor(p.w)
    }
    fn project_3d_to_2d(&self, p: cgmath::Point3<f32>) -> cgmath::Point2<f32> {
        cgmath::point2(p.x, p.y) / self.z_divisor(p.z)
    }
    fn scale_screen_space_to_ndc(&self, p: cgmath::Point2<f32>) -> Option<cgmath::Point2<f32>> {
        let xy_scale = self.xy_scale().ok()?;
        let x = p.x * xy_scale.x;
        let y = p.y * xy_scale.y;
        Some(cgmath::point2(x, y))
    }
    /// Projects an N-dimensional vector `v` to a 2D vector in screen space.
    /// Because the perspective projection is nonlinear, this also requires an
    /// initial point `p` where the vector `v` originates.
    pub fn project_vector_to_screen_space(
        &self,
        p: &Point,
        v: &Vector,
    ) -> Option<cgmath::Vector2<f32>> {
        // This mimics a similar function in the WGSL shader.

        // Rotate.
        let p = self.rot.transform(p);
        let v = self.rot.transform(v);

        // Convert to cgmath vector.
        let p_4d = hypermath_to_cgmath_vec4(p.as_vector());
        let v_4d = hypermath_to_cgmath_vec4(v);

        // Apply 4D perspective transformation.
        let p_3d = self.project_4d_to_3d(p_4d);
        let v_3d = (v_4d.truncate() + p_3d.to_vec() * v_4d.w * self.w_factor_4d())
            / self.w_divisor(p_4d.w);

        // Apply 3D perspective transformation.
        let p_2d = self.project_3d_to_2d(p_3d);
        let v_2d = (v_3d.truncate() + p_2d.to_vec() * p_3d.z * self.w_factor_3d())
            / self.z_divisor(p_3d.z);

        Some(v_2d)
    }

    /// Returns the W coordinate of the 4D camera in N-dimensional global space.
    pub fn camera_4d_w(&self) -> f32 {
        1.0 + 1.0 / self.w_factor_4d()
    }
    /// Returns the position of the 4D camera in N-dimensional puzzle space.
    pub fn camera_4d_pos(&self) -> Point {
        let global_camera_4d_pos = point![0.0, 0.0, 0.0, self.camera_4d_w() as Float];
        self.rot.reverse().transform(&global_camera_4d_pos)
    }
}

fn hypermath_to_cgmath_vec4(v: impl VectorRef) -> cgmath::Vector4<f32> {
    cgmath::vec4(v.get(0) as _, v.get(1) as _, v.get(2) as _, v.get(3) as _)
}
