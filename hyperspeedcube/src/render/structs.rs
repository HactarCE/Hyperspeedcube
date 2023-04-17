//! Structs shared between the CPU and GPU (vertices, uniforms, etc.).

#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub(super) struct GfxProjectionParams {
    pub facet_scale: f32,
    pub sticker_scale: f32,
    pub w_factor_4d: f32,
    pub w_factor_3d: f32,
    pub fov_signum: f32,
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub(super) struct BasicVertex {
    pub pos: [f32; 3],
    pub color: [f32; 3],
}
impl BasicVertex {
    pub const LAYOUT: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &wgpu::vertex_attr_array![
            0 => Float32x3,
            1 => Float32x3,
        ],
    };
}
