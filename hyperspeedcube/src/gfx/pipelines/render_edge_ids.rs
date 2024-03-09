use super::*;

pipeline!(pub(in crate::gfx) struct Pipeline {
    type = wgpu::RenderPipeline;

    struct Bindings<'a> {
        edge_verts:          &'a wgpu::Buffer = pub(VERTEX) bindings::EDGE_VERTS,
        vertex_3d_positions: &'a wgpu::Buffer = pub(VERTEX) bindings::VERTEX_3D_POSITIONS_READONLY,
        vertex_culls:        &'a wgpu::Buffer = pub(VERTEX) bindings::VERTEX_CULLS_READONLY,

        draw_params:         &'a wgpu::Buffer = pub(VERTEX_FRAGMENT) bindings::DRAW_PARAMS,
    }

    let pipeline_descriptor = RenderPipelineDescriptor {
        label: "render_edge_ids",
        vertex_buffers: &[
            single_type_vertex_buffer![for Instance, 0 => Sint32], // edge_id
        ],
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleStrip,
            ..Default::default()
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: wgpu::TextureFormat::Depth32Float,
            depth_write_enabled: true,
            depth_compare: wgpu::CompareFunction::Greater,
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        fragment_target: Some(wgpu::ColorTargetState {
            format: wgpu::TextureFormat::R32Uint,
            blend: None,
            write_mask: wgpu::ColorWrites::ALL,
        }),
        ..Default::default()
    };
});

pub type PassParams<'tex> = super::render_polygon_ids::PassParams<'tex>;
