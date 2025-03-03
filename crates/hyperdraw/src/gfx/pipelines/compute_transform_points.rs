use super::*;

pipeline!(pub(in crate::gfx) struct Pipeline {
    type = wgpu::ComputePipeline;

    struct Bindings<'a> {
        vertex_positions:       &'a wgpu::Buffer = pub(COMPUTE) bindings::VERTEX_POSITIONS,
        u_tangents:             &'a wgpu::Buffer = pub(COMPUTE) bindings::U_TANGENTS,
        v_tangents:             &'a wgpu::Buffer = pub(COMPUTE) bindings::V_TANGENTS,
        sticker_shrink_vectors: &'a wgpu::Buffer = pub(COMPUTE) bindings::STICKER_SHRINK_VECTORS,
        piece_ids:              &'a wgpu::Buffer = pub(COMPUTE) bindings::PIECE_IDS,
        surface_ids:            &'a wgpu::Buffer = pub(COMPUTE) bindings::SURFACE_IDS,

        piece_centroids:        &'a wgpu::Buffer = pub(COMPUTE) bindings::PIECE_CENTROIDS,
        surface_centroids:      &'a wgpu::Buffer = pub(COMPUTE) bindings::SURFACE_CENTROIDS,
        surface_normals:        &'a wgpu::Buffer = pub(COMPUTE) bindings::SURFACE_NORMALS,
        vertex_3d_positions:    &'a wgpu::Buffer = pub(COMPUTE) bindings::VERTEX_3D_POSITIONS,
        vertex_3d_normals:      &'a wgpu::Buffer = pub(COMPUTE) bindings::VERTEX_3D_NORMALS,

        puzzle_transform:       &'a wgpu::Buffer = pub(COMPUTE) bindings::PUZZLE_TRANSFORM,
        piece_transforms:       &'a wgpu::Buffer = pub(COMPUTE) bindings::PIECE_TRANSFORMS,
        camera_4d_pos:          &'a wgpu::Buffer = pub(COMPUTE) bindings::CAMERA_4D_POS,
        draw_params:            &'a wgpu::Buffer = pub(COMPUTE) bindings::DRAW_PARAMS,
    }

    let pipeline_descriptor = ComputePipelineDescriptor {
        label: "compute_transform_points",
        entry_point: "compute_transform_points",
    };
});

pub const PASS_DESCRIPTOR: wgpu::ComputePassDescriptor<'static> = wgpu::ComputePassDescriptor {
    label: Some("compute_transform_points"),
    timestamp_writes: None,
};
