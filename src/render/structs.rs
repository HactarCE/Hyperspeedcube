//! Structs shared between the CPU and GPU (vertices, uniforms, etc.).

use cgmath::{Point2, Point3, Vector3};
use smallvec::SmallVec;

use super::util::IterCyclicPairsExt;

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
    pub sticker_id: usize,

    pub verts: Box<[Point3<f32>]>,
    pub min_bound: Point3<f32>,
    pub max_bound: Point3<f32>,

    pub front_polygons: Box<[Polygon]>,
    pub back_polygons: Box<[Polygon]>,

    pub outlines: Box<[[u16; 2]]>,
    pub outline_color: [f32; 4],
    pub outline_size: f32,
}
impl ProjectedStickerGeometry {
    pub(super) fn contains_point(&self, point: Point2<f32>) -> bool {
        self.front_polygons
            .iter()
            .any(|polygon| polygon.contains_point(point))
    }
}

#[derive(Debug, Clone)]
pub(super) struct Polygon {
    pub verts: SmallVec<[Point3<f32>; 4]>,
    pub min_bound: Point3<f32>,
    pub max_bound: Point3<f32>,
    pub normal: Vector3<f32>,

    pub color: [f32; 4],
}
impl Polygon {
    fn contains_point(&self, point: Point2<f32>) -> bool {
        // println!("{:?}", (self.min_bound.x, self.min_bound.y));
        self.min_bound.x <= point.x
            && self.min_bound.y <= point.y
            && point.x <= self.max_bound.x
            && point.y <= self.max_bound.y
            && self
                .verts
                .iter()
                .map(|v| cgmath::point2(v.x, v.y))
                .cyclic_pairs()
                .all(|(a, b)| (b - a).perp_dot(point - a) <= 0.0)
    }
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub(super) struct BasicUniform {
    pub scale: [f32; 2],
}
