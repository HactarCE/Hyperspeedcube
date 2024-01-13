use super::*;

pipeline!(pub(in crate::gfx) struct Pipeline {
    type = wgpu::ComputePipeline;

    struct Bindings<'a> {
        vertex_positions:       &'a wgpu::Buffer = pub(COMPUTE) bindings::VERTEX_POSITIONS,
        u_tangents:             &'a wgpu::Buffer = pub(COMPUTE) bindings::U_TANGENTS,
        v_tangents:             &'a wgpu::Buffer = pub(COMPUTE) bindings::V_TANGENTS,
        sticker_shrink_vectors: &'a wgpu::Buffer = pub(COMPUTE) bindings::STICKER_SHRINK_VECTORS,
        piece_ids:              &'a wgpu::Buffer = pub(COMPUTE) bindings::PIECE_IDS,
        facet_ids:              &'a wgpu::Buffer = pub(COMPUTE) bindings::FACET_IDS,
        piece_centroids:        &'a wgpu::Buffer = pub(COMPUTE) bindings::PIECE_CENTROIDS,
        facet_centroids:        &'a wgpu::Buffer = pub(COMPUTE) bindings::FACET_CENTROIDS,
        facet_normals:          &'a wgpu::Buffer = pub(COMPUTE) bindings::FACET_NORMALS,
        vertex_3d_positions:    &'a wgpu::Buffer = pub(COMPUTE) bindings::VERTEX_3D_POSITIONS,
        vertex_lightings:       &'a wgpu::Buffer = pub(COMPUTE) bindings::VERTEX_LIGHTINGS,
        vertex_culls:           &'a wgpu::Buffer = pub(COMPUTE) bindings::VERTEX_CULLS,
        puzzle_transform:       &'a wgpu::Buffer = pub(COMPUTE) bindings::PUZZLE_TRANSFORM,
        piece_transforms:       &'a wgpu::Buffer = pub(COMPUTE) bindings::PIECE_TRANSFORMS,
        camera_4d_pos:          &'a wgpu::Buffer = pub(COMPUTE) bindings::CAMERA_4D_POS,
        projection_params:      &'a wgpu::Buffer = pub(COMPUTE) bindings::PROJECTION_PARAMS,
        lighting_params:        &'a wgpu::Buffer = pub(COMPUTE) bindings::LIGHTING_PARAMS,
        view_params:            &'a wgpu::Buffer = pub(COMPUTE) bindings::VIEW_PARAMS,
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
