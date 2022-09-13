//! Structs shared between the CPU and GPU (vertices, uniforms, etc.).

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub(super) struct RgbaVertex {
    pub pos: [f32; 3],
    pub color: [f32; 4],
}
impl RgbaVertex {
    pub const LAYOUT: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &wgpu::vertex_attr_array![
            0 => Float32x3,
            1 => Float32x4,
        ],
    };
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub(super) struct BasicUniform {
    pub scale: [f32; 2],
    pub align: [f32; 2],
}
