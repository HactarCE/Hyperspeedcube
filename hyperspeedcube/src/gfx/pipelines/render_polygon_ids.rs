use super::*;

pipeline!(pub(in crate::gfx) struct Pipeline {
    type = wgpu::RenderPipeline;

    struct Bindings<'a> {
        projection_params: &'a wgpu::Buffer = pub(VERTEX) bindings::PROJECTION_PARAMS,
        view_params:       &'a wgpu::Buffer = pub(VERTEX) bindings::VIEW_PARAMS, // TODO: shouldn't need view_params
    }

    let pipeline_descriptor = RenderPipelineDescriptor {
        label: "render_polygon_ids",
        vertex_buffers: &[
            single_type_vertex_buffer![0 => Float32x4], // position
            single_type_vertex_buffer![1 => Float32x4], // normal
            single_type_vertex_buffer![2 => Sint32],    // polygon_id
            single_type_vertex_buffer![3 => Float32],   // cull
        ],
        depth_stencil: Some(wgpu::DepthStencilState {
            format: wgpu::TextureFormat::Depth32Float,
            depth_write_enabled: true,
            depth_compare: wgpu::CompareFunction::Greater,
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        fragment_target: Some(wgpu::ColorTargetState {
            format: wgpu::TextureFormat::Rg32Uint,
            blend: None,
            write_mask: wgpu::ColorWrites::ALL,
        }),
        ..Default::default()
    };
});

pub(in crate::gfx) struct PassParams<'tex> {
    pub clear: bool,
    pub ids_texture: &'tex wgpu::TextureView,
    pub ids_depth_texture: &'tex wgpu::TextureView,
}
impl<'pass> PassParams<'pass> {
    pub fn begin_pass(self, encoder: &'pass mut wgpu::CommandEncoder) -> wgpu::RenderPass<'pass> {
        encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("render_ids"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: self.ids_texture,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: match self.clear {
                        true => wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        false => wgpu::LoadOp::Load,
                    },
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: self.ids_depth_texture,
                depth_ops: Some(wgpu::Operations {
                    load: match self.clear {
                        true => wgpu::LoadOp::Clear(0.0),
                        false => wgpu::LoadOp::Load,
                    },
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            ..Default::default()
        })
    }
}
