use super::*;

pipeline!(pub(in crate::gfx) struct Pipeline {
    type = wgpu::RenderPipeline;

    struct Bindings<'a> {
        src_texture:   &'a wgpu::TextureView = pub(FRAGMENT) bindings::BLIT_SRC_TEXTURE,
        src_sampler:   &'a wgpu::Sampler     = pub(FRAGMENT) bindings::BLIT_SRC_SAMPLER,
        effect_params: &'a wgpu::Buffer      = pub(FRAGMENT) bindings::EFFECT_PARAMS,
    }

    struct PipelineParams {
        target_format: wgpu::TextureFormat,
        premultiply_alpha: bool,
    }
    let pipeline_descriptor = RenderPipelineDescriptor {
        vertex_entry_point: "uv_vertex",
        fragment_entry_point: if premultiply_alpha {
            "blit_fragment"
        } else {
            "blit_fragment_unmultiply_alpha"
        },
        vertex_buffers: &[UvVertex::LAYOUT],
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleStrip,
            ..Default::default()
        },
        fragment_target: Some(wgpu::ColorTargetState {
            format: target_format,
            blend: Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
            write_mask: wgpu::ColorWrites::all(),
        }),
        ..Default::default()
    };
});
