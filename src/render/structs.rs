//! Structs shared between the CPU and GPU (vertices, uniforms, etc.).

use cgmath::{Point3, Vector3};
use smallvec::SmallVec;

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

#[derive(Debug)]
pub(super) struct ProjectedStickerGeometry {
    pub verts: Box<[Point3<f32>]>,
    pub min_bound: Point3<f32>,
    pub max_bound: Point3<f32>,

    pub front_polygons: Box<[Polygon]>,
    pub back_polygons: Box<[Polygon]>,

    pub outlines: Box<[[u16; 2]]>,
    pub outline_color: [f32; 4],
}

#[derive(Debug, Clone)]
pub(super) struct Polygon {
    pub verts: SmallVec<[Point3<f32>; 4]>,
    pub min_bound: Point3<f32>,
    pub max_bound: Point3<f32>,
    pub normal: Vector3<f32>,

    pub color: [f32; 4],
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub(super) struct BasicUniform {
    pub scale: [f32; 2],
}
