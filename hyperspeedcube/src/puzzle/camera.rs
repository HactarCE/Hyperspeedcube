use cgmath::EuclideanSpace;
use eyre::{bail, Result};
use hypermath::prelude::*;

use crate::preferences::ViewPreferences;

/// Parameters controlling the camera and lighting.
#[derive(Debug, Clone, PartialEq)]
pub struct Camera {
    pub prefs: ViewPreferences,

    /// Width and height of the target in pixels.
    pub target_size: [u32; 2],

    pub rot: pga::Motor,
    pub zoom: f32,
}
impl Camera {
    /// Returns the number of pixels in 1 screen space unit.
    fn compute_pixel_scale(target_size: [u32; 2], zoom: f32) -> Result<f32> {
        let w = target_size[0] as f32;
        let h = target_size[1] as f32;
        let min_dimen = f32::min(w as f32, h as f32);
        if min_dimen == 0.0 {
            bail!("puzzle view has zero size");
        }
        Ok(min_dimen * zoom)
    }
    /// Returns the size of a pixel in screen space.
    pub fn compute_pixel_size(target_size: [u32; 2], zoom: f32) -> Result<f32> {
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

    /// Returns the factor by which the W coordinate affects the XYZ coordinates
    /// during 4D projection.
    pub fn w_factor_4d(&self) -> f32 {
        (self.prefs.fov_4d.to_radians() * 0.5).tan()
    }
    /// Returns the factor by which the Z coordinate affects the XY coordinates
    /// during 3D projection.
    pub fn w_factor_3d(&self) -> f32 {
        (self.prefs.fov_3d.to_radians() * 0.5).tan()
    }
    /// Returns the 4D perspective divisor based on the W coordinate of a point.
    pub fn w_divisor(&self, w: f32) -> f32 {
        1.0 + w * self.w_factor_4d()
    }
    /// Returns the 3D perspective divisor based on the Z coordinate of a point.
    pub fn z_divisor(&self, z: f32) -> f32 {
        1.0 + (self.prefs.fov_3d.signum() - z) * self.w_factor_3d()
    }
    /// Projects an N-dimensional point to a 2D point on the screen.
    pub fn project_point(&self, p: impl VectorRef) -> Option<cgmath::Point2<f32>> {
        // This mimics a similar function in the WGSL shader.
        let p = self.rot.transform_point(p); // Rotate
        let p = hypermath_to_cgmath_vec4(p); // Convert to cgmath vector
        let p = self.project_4d_to_3d(p); // Apply 4D perspective transformation
        let p = self.project_3d_to_2d(p); // Apply 3D perspective transformation
        self.scale_2d_world_to_screen_space(p.to_vec()) // Apply scaling
            .map(cgmath::Point2::from_vec)
    }
    fn project_4d_to_3d(&self, p: cgmath::Vector4<f32>) -> cgmath::Point3<f32> {
        // Offset the camera to W = -1 and apply 4D perspective transformation.
        let w = p.w as f32 + 1.0;
        cgmath::Point3::from_vec(p.truncate() / self.w_divisor(w))
    }
    fn project_3d_to_2d(&self, p: cgmath::Point3<f32>) -> cgmath::Point2<f32> {
        let z = p.z as f32;
        cgmath::Point2::from_vec(p.to_vec().truncate() / self.z_divisor(z))
    }
    fn scale_2d_world_to_screen_space(
        &self,
        p: cgmath::Vector2<f32>,
    ) -> Option<cgmath::Vector2<f32>> {
        let xy_scale = self.xy_scale().ok()?;
        let x = p.x as f32 * xy_scale.x;
        let y = p.y as f32 * xy_scale.y;
        Some(cgmath::vec2(x, y))
    }
    /// Projects an N-dimensional vector `v` to a 2D vector on the screen.
    /// Because the perspective projection is nonlinear, this also requires an
    /// initial point `p` where the vector `v` originates.
    pub fn project_vector(
        &self,
        p: impl VectorRef,
        v: impl VectorRef,
    ) -> Option<cgmath::Vector2<f32>> {
        // This mimics a similar function in the WGSL shader.

        // Rotate.
        let p = self.rot.transform_point(p);
        let v = self.rot.transform_vector(v);

        // Convert to cgmath vector.
        let p_4d = hypermath_to_cgmath_vec4(p);
        let v_4d = hypermath_to_cgmath_vec4(v);

        // Apply 4D perspective transformation.
        let p_3d = self.project_4d_to_3d(p_4d);
        let v_3d = (v_4d.truncate() - p_3d.to_vec() * v_4d.w * self.w_factor_4d())
            / self.w_divisor(p_4d.w);

        // Apply 3D perspective transformation.
        let p_2d = self.project_3d_to_2d(p_3d);
        let v_2d = (v_3d.truncate() + p_2d.to_vec() * p_3d.z * self.w_factor_3d())
            / self.z_divisor(p_3d.z);

        self.scale_2d_world_to_screen_space(v_2d)
    }
}

fn hypermath_to_cgmath_vec4(v: impl VectorRef) -> cgmath::Vector4<f32> {
    cgmath::vec4(v.get(0) as _, v.get(1) as _, v.get(2) as _, v.get(3) as _)
}
