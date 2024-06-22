//! Structs shared between the CPU and GPU (vertices, uniforms, etc.).

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, PartialEq, bytemuck::NoUninit, bytemuck::Zeroable)]
pub(super) struct UvVertex {
    pub position: [f32; 2],
    pub uv: [f32; 2],
}
impl UvVertex {
    const fn new(position: [f32; 2], uv: [f32; 2]) -> Self {
        Self { position, uv }
    }

    pub const LAYOUT: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &wgpu::vertex_attr_array![
            0 => Float32x2,
            1 => Float32x2,
        ],
    };
    pub const SQUARE: [Self; 4] = [
        UvVertex::new([-1.0, 1.0], [0.0, 0.0]),
        UvVertex::new([1.0, 1.0], [1.0, 0.0]),
        UvVertex::new([-1.0, -1.0], [0.0, 1.0]),
        UvVertex::new([1.0, -1.0], [1.0, 1.0]),
    ];
}

/// Precomputed intermediate values for graphics calculations.
///
/// - `n` = near plane Z coordinate
/// - `f` = far plane Z coordinate
/// - `fpzd` = far plane Z divisor = `z_divisor(f)`
/// - `npzd` = near plane Z divisor = `z_divisor(n)`
/// - `z0zd` = Z divisor at Z=0 = `z_divisor(0.0)`
/// -
#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq, bytemuck::NoUninit, bytemuck::Zeroable)]
pub struct GfxPrecomputedValues {
    /// Near plane Z coordinate.
    pub n: f32,
    /// Far plane Z coordinate.
    pub f: f32,

    /// Near plane Z divisor: `z_divisor(near_plane_z)`
    pub npzd: f32,
    /// Far plane Z divisor: `z_divisor(far_plane_z)`
    pub fpzd: f32,
    /// Z divisor at Z=0: `z_divisor(0.0)`
    pub z0zd: f32,

    /// `z_divisor(near_plane_z) * z_divisor(far_plane_z)`
    pub npzd_fpzd: f32,

    /// `w_factor_3d * fov_signum + 1.0`
    pub wf_s_plus_1: f32,

    /// `n * z_divisor(far_plane_z)`
    pub n_fpzd: f32,
    /// `w_factor_3d * z_divisor(far_plane_z)`
    pub wf_fpzd: f32,

    /// `n - f`
    pub nf: f32,
    /// `(n - f) * wf`
    pub nf_wf: f32,
    /// `(n - f) * z_divisor(0.0)`
    pub nf_z0zd: f32,
}
impl GfxPrecomputedValues {
    pub fn new(w_factor_3d: f32, near_plane_z: f32, far_plane_z: f32) -> Self {
        let wf = w_factor_3d;

        let n = near_plane_z;
        let f = far_plane_z;

        let s = w_factor_3d.signum();

        let z_div = |z| wf * (s - z) + 1.0;

        let npzd = z_div(n);
        let fpzd = z_div(f);
        let z0zd = z_div(0.0);

        let npzd_fpzd = npzd * fpzd;

        let wf_s_plus_1 = wf * s + 1.0;

        let n_fpzd = n * fpzd;
        let wf_fpzd = wf * fpzd;

        let nf = n - f;
        let nf_wf = nf * wf;
        let nf_z0zd = nf * z0zd;

        Self {
            n,
            f,

            npzd,
            fpzd,
            z0zd,

            npzd_fpzd,

            wf_s_plus_1,

            n_fpzd,
            wf_fpzd,

            nf,
            nf_wf,
            nf_z0zd,
        }
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq, bytemuck::NoUninit, bytemuck::Zeroable)]
pub struct GfxDrawParams {
    pub pre: GfxPrecomputedValues,

    /// Vector indicating the direction that light is shining from.
    pub light_dir: [f32; 3],
    /// Intensity of directional light for faces.
    pub face_light_intensity: f32,
    /// Intensity of directional light for outlines.
    pub outline_light_intensity: f32,

    /// Width and height of a pixel in screen space.
    pub pixel_size: f32,
    /// Width and height of the target in pixels.
    pub target_size: [f32; 2],
    /// 2D X & Y scale factors to apply after perspective transformation.
    pub xy_scale: [f32; 2],

    /// Mouse cursor position in NDC (normalized device coordinates).
    pub cursor_pos: [f32; 2],

    pub facet_shrink: f32,
    pub sticker_shrink: f32,
    pub piece_explode: f32,

    pub near_plane_z: f32,
    pub far_plane_z: f32,
    pub w_factor_4d: f32,
    pub w_factor_3d: f32,
    pub fov_signum: f32,
    /// Whether to show frontfaces. (`bool`)
    pub show_frontfaces: i32,
    /// Whether to show backfaces. (`bool`)
    pub show_backfaces: i32,
    /// Whether to clip 4D geometry behind the camera. (`bool`)
    pub clip_4d_behind_camera: i32,
    /// W coordinate of the 4D camera.
    pub camera_4d_w: f32,
}
