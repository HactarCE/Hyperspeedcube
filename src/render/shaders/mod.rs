#[derive(Default)]
pub(super) struct Shaders {
    basic: Option<wgpu::ShaderModule>,
}
impl Shaders {
    pub(super) fn new() -> Self {
        Self::default()
    }
    pub(super) fn basic(&mut self, device: &wgpu::Device) -> &wgpu::ShaderModule {
        self.basic
            .get_or_insert_with(|| device.create_shader_module(&wgpu::include_wgsl!("basic.wgsl")))
    }
}
