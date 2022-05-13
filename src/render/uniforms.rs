#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub(super) struct PuzzleUniform {
    pub transform: [[f32; 4]; 4],
    pub light_direction: [f32; 3],
    pub min_light: f32,
}
