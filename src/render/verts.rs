#[derive(Debug, Default, Copy, Clone)]
pub struct RgbaVertex {
    pub pos: [f32; 4],
    pub color: [f32; 4],
}
implement_vertex!(RgbaVertex, pos, color);
