use once_cell::unsync::OnceCell;

use super::GraphicsState;

pub(super) struct Shaders {
    pub(super) polygon_ids: CachedShaderModule,
    pub(super) color: CachedShaderModule,
}
impl Shaders {
    pub(super) fn new() -> Self {
        Self {
            polygon_ids: CachedShaderModule::new(|| wgpu::include_wgsl!("polygon_ids.wgsl")),
            color: CachedShaderModule::new(|| wgpu::include_wgsl!("color.wgsl")),
        }
    }
}

pub(super) struct CachedShaderModule {
    // TODO: when https://github.com/gfx-rs/wgpu/pull/2902 is merged, don't use
    // a function pointer here.
    desc: fn() -> wgpu::ShaderModuleDescriptor<'static>,
    shader: OnceCell<wgpu::ShaderModule>,
}
impl CachedShaderModule {
    fn new(desc: fn() -> wgpu::ShaderModuleDescriptor<'static>) -> Self {
        Self {
            desc,
            shader: OnceCell::new(),
        }
    }
    pub(super) fn get(&self, gfx: &GraphicsState) -> &wgpu::ShaderModule {
        self.shader
            .get_or_init(|| gfx.device.create_shader_module((self.desc)()))
    }
}
