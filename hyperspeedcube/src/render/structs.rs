//! Structs shared between the CPU and GPU (vertices, uniforms, etc.).

#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub(super) struct GfxLightingParams {
    pub dir: [f32; 3],
    pub ambient: f32,
    pub directional: f32,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub(super) struct GfxProjectionParams {
    pub facet_shrink: f32,
    pub sticker_shrink: f32,
    pub piece_explode: f32,

    pub w_factor_4d: f32,
    pub w_factor_3d: f32,
    pub fov_signum: f32,
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub(super) struct GfxCompositeParams {
    pub alpha: f32,
    pub outline_radius: u32,
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub(super) struct GfxSpecialColors {
    pub background: [f32; 3],
    pub _padding1: u32,
    pub outline: [f32; 3],
    pub _padding2: u32,
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub(super) struct CompositeVertex {
    pub position: [f32; 2],
    pub uv: [f32; 2],
}
impl CompositeVertex {
    pub const LAYOUT: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &wgpu::vertex_attr_array![
            0 => Float32x2,
            1 => Float32x2,
        ],
    };
    pub const SQUARE: [Self; 4] = [
        CompositeVertex {
            position: [-1.0, 1.0],
            uv: [0.0, 0.0],
        },
        CompositeVertex {
            position: [1.0, 1.0],
            uv: [1.0, 0.0],
        },
        CompositeVertex {
            position: [-1.0, -1.0],
            uv: [0.0, 1.0],
        },
        CompositeVertex {
            position: [1.0, -1.0],
            uv: [1.0, 1.0],
        },
    ];
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

#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GfxViewParams {
    pub scale: [f32; 2],
    pub align: [f32; 2],
}
