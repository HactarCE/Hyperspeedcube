use super::*;

pipeline!(pub(in crate::gfx) struct Pipeline {
    type = wgpu::RenderPipeline;

    struct Bindings<'a> {
        src_texture: &'a wgpu::TextureView = pub(FRAGMENT) bindings::BLIT_SRC_TEXTURE,
        src_sampler: &'a wgpu::Sampler = pub(FRAGMENT) bindings::BLIT_SRC_SAMPLER,
    }

    struct PipelineParams {
        target_format: wgpu::TextureFormat,
    }
    let pipeline_descriptor = RenderPipelineDescriptor {
        vertex_entry_point: "uv_vertex",
        fragment_entry_point: "blit_fragment",
        vertex_buffers: &[UvVertex::LAYOUT],
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleStrip,
            ..Default::default()
        },
        fragment_target: Some(wgpu::ColorTargetState::from(target_format)),
        ..Default::default()
    };
});
