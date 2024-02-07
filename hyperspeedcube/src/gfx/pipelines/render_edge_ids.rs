use super::*;

pipeline!(pub(in crate::gfx) struct Pipeline {
    type = wgpu::RenderPipeline;

    struct Bindings<'a> {
        edge_verts:          &'a wgpu::Buffer = pub(VERTEX) bindings::EDGE_VERTS,
        vertex_3d_positions: &'a wgpu::Buffer = pub(VERTEX) bindings::VERTEX_3D_POSITIONS_READONLY,
        vertex_culls:        &'a wgpu::Buffer = pub(VERTEX) bindings::VERTEX_CULLS_READONLY,
        projection_params:   &'a wgpu::Buffer = pub(VERTEX_FRAGMENT) bindings::PROJECTION_PARAMS,
        view_params:         &'a wgpu::Buffer = pub(VERTEX_FRAGMENT) bindings::VIEW_PARAMS, // TODO: shouldn't need view_params
        target_size:         &'a wgpu::Buffer = pub(VERTEX_FRAGMENT) bindings::TARGET_SIZE,
    }

    let pipeline_descriptor = RenderPipelineDescriptor {
        label: "render_edge_ids",
        vertex_buffers: &[
            single_type_vertex_buffer![for Instance, 0 => Sint32], // edge_id
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
            write_mask: wgpu::ColorWrites::GREEN,
        }),
        ..Default::default()
    };
});
