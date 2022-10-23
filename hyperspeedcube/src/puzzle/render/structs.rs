//! Structs shared between the CPU and GPU (vertices, uniforms, etc.).

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub(super) struct GfxViewParams {
    pub scale: [f32; 2],
    pub align: [f32; 2],
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub(super) struct CompositeVertex {
    pub pos: [f32; 2],
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
}

#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, bytemuck::Pod, bytemuck::Zeroable)]
pub(super) struct PolygonVertex {
    pub polygon: i32,
    pub vertex: u32,
}
impl PolygonVertex {
    pub const LAYOUT: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &wgpu::vertex_attr_array![
            0 => Sint32,
            1 => Uint32,
        ],
    };
}

#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, bytemuck::Pod, bytemuck::Zeroable)]
pub(super) struct GfxStickerInfo {
    pub piece: u32,
    pub facet: u32,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, bytemuck::Pod, bytemuck::Zeroable)]
pub(super) struct GfxPolygonInfo {
    pub facet: u32,
    pub v0: u32,
    pub v1: u32,
    pub v2: u32,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub(super) struct GfxProjectionParams {
    pub facet_scale: f32,
    pub sticker_scale: f32,
    pub w_factor_4d: f32,
    pub w_factor_3d: f32,
    pub fov_signum: f32,
    pub ndim: u32,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub(super) struct GfxLightingParams {
    pub dir: [f32; 3],
    pub ambient: f32,
    pub directional: f32,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub(super) struct GfxCompositeParams {
    pub background_color: [f32; 3],
    pub alpha: f32,
    pub outline_color: [f32; 3],
    pub outline_radius: u32,
}
