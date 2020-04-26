#[derive(Debug, Default, Copy, Clone)]
pub struct StickerVertex {
    pub pos: [f32; 4],
    pub color: [f32; 4],
}
implement_vertex!(StickerVertex, pos, color);
