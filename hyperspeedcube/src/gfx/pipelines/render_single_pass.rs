use super::*;

pipeline!(pub(in crate::gfx) struct Pipeline {
    type = wgpu::RenderPipeline;

    struct Bindings<'a> {
        vertex_positions:       &'a wgpu::Buffer = pub(VERTEX) bindings::VERTEX_POSITIONS,
        u_tangents:             &'a wgpu::Buffer = pub(VERTEX) bindings::U_TANGENTS,
        v_tangents:             &'a wgpu::Buffer = pub(VERTEX) bindings::V_TANGENTS,
        sticker_shrink_vectors: &'a wgpu::Buffer = pub(VERTEX) bindings::STICKER_SHRINK_VECTORS,

        piece_centroids:        &'a wgpu::Buffer = pub(VERTEX) bindings::PIECE_CENTROIDS,
        facet_centroids:        &'a wgpu::Buffer = pub(VERTEX) bindings::FACET_CENTROIDS,
        facet_normals:          &'a wgpu::Buffer = pub(VERTEX) bindings::FACET_NORMALS,

        puzzle_transform:       &'a wgpu::Buffer = pub(VERTEX) bindings::PUZZLE_TRANSFORM,
        piece_transforms:       &'a wgpu::Buffer = pub(VERTEX) bindings::PIECE_TRANSFORMS,
        camera_4d_pos:          &'a wgpu::Buffer = pub(VERTEX) bindings::CAMERA_4D_POS,
        polygon_color_ids:      &'a wgpu::Buffer = pub(FRAGMENT) bindings::POLYGON_COLOR_IDS,
        draw_params:            &'a wgpu::Buffer = pub(VERTEX) bindings::DRAW_PARAMS,

        colors_texture:         &'a wgpu::TextureView = pub(FRAGMENT) bindings::COLORS_TEXTURE,
    }

    let pipeline_descriptor = RenderPipelineDescriptor {
        label: "render_single_pass",
        vertex_buffers: &[
            single_type_vertex_buffer![0 => Sint32], // piece_id
            single_type_vertex_buffer![1 => Sint32], // facet_id
            single_type_vertex_buffer![2 => Sint32], // polygon_id
        ],
        depth_stencil: Some(wgpu::DepthStencilState {
            format: wgpu::TextureFormat::Depth32Float,
            depth_write_enabled: true,
            depth_compare: wgpu::CompareFunction::Greater,
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        fragment_target: Some(wgpu::ColorTargetState {
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            blend: Some(wgpu::BlendState::REPLACE),
            write_mask: wgpu::ColorWrites::ALL,
        }),
        ..Default::default()
    };
});

pub(in crate::gfx) struct PassParams<'tex> {
    pub clear_color: [f64; 3],
    pub color_texture: &'tex wgpu::TextureView,
    pub depth_texture: &'tex wgpu::TextureView,
}
impl<'pass> PassParams<'pass> {
    pub(in crate::gfx) fn begin_pass(
        self,
        encoder: &'pass mut wgpu::CommandEncoder,
    ) -> wgpu::RenderPass<'pass> {
        let [r, g, b] = self.clear_color;
        encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("render_puzzle"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: self.color_texture,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color { r, g, b, a: 1.0 }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: self.depth_texture,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(0.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            ..Default::default()
        })
    }
}
