use super::*;

pipeline!(pub(in crate::gfx) struct Pipeline {
    type = wgpu::RenderPipeline;

    struct Bindings<'a> {
        polygon_color_ids: &'a wgpu::Buffer = pub(VERTEX) bindings::POLYGON_COLOR_IDS,
        draw_params:       &'a wgpu::Buffer = pub(VERTEX) bindings::DRAW_PARAMS,
    }

    let pipeline_descriptor = RenderPipelineDescriptor {
        label: "render_polygons",
        vertex_buffers: &[
            single_type_vertex_buffer![0 => Float32x4], // position
            single_type_vertex_buffer![1 => Float32x4], // normal
            single_type_vertex_buffer![2 => Sint32],    // polygon_id
        ],
        depth_stencil: Some(wgpu::DepthStencilState {
            format: wgpu::TextureFormat::Depth32Float,
            depth_write_enabled: true,
            depth_compare: wgpu::CompareFunction::Less,
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
                depth_slice: None,
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
                        true => wgpu::LoadOp::Clear(1.0),
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
