use cgmath::Vector3;

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct RgbaVertex {
    pub pos: [f32; 4],
    pub normal: [f32; 4],
    pub color: [f32; 4],
}
impl RgbaVertex {
    pub const LAYOUT: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &wgpu::vertex_attr_array![
            0 => Float32x4,
            1 => Float32x4,
            2 => Float32x4,
        ],
    };

    pub fn tri(points: [Vector3<f32>; 3], color: [f32; 4]) -> impl Iterator<Item = Self> {
        let normal = (points[1] - points[0]).cross(points[2] - points[0]);
        points.into_iter().map(move |p| Self {
            pos: p.extend(1.0).into(),
            normal: normal.extend(0.0).into(),
            color,
        })
    }
    pub fn quad(points: [Vector3<f32>; 4], color: [f32; 4]) -> impl Iterator<Item = Self> {
        let t1 = Self::tri([points[2], points[1], points[0]], color);
        let t2 = Self::tri([points[1], points[2], points[3]], color);
        t1.chain(t2)
    }
    pub fn double_quad(
        [a, b, c, d]: [Vector3<f32>; 4],
        color: [f32; 4],
    ) -> impl Iterator<Item = Self> {
        let q1 = Self::quad([a, b, c, d], color);
        let q2 = Self::quad([a, c, b, d], color);
        q1.chain(q2)
    }
    pub fn cube(
        [a, b, c, d, e, f, g, h]: [Vector3<f32>; 8],
        color: [f32; 4],
    ) -> impl Iterator<Item = Self> {
        let q1 = Self::quad([a, b, c, d], color);
        let q2 = Self::quad([e, g, f, h], color);
        let q3 = Self::quad([a, c, e, g], color);
        let q4 = Self::quad([b, f, d, h], color);
        let q5 = Self::quad([a, e, b, f], color);
        let q6 = Self::quad([c, d, g, h], color);
        q1.chain(q2).chain(q3).chain(q4).chain(q5).chain(q6)
    }
}
