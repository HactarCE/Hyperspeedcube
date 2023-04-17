struct PolygonVertex {
    @location(0) polygon_id: i32,
    @location(1) vertex_id: u32,
}

struct VertexOutput {
    @builtin(position) pos: vec4<f32>,
    @location(0) polygon_id: i32,
}

struct ViewParams {
    scale: vec2<f32>,
    align: vec2<f32>,
}

@group(0) @binding(0) var<uniform> view_params: ViewParams;

@group(0) @binding(1) var<storage, read> vertex_3d_position_array: array<vec4<f32>>;

@vertex
fn vs_main(
    in: PolygonVertex,
    @builtin(vertex_index) idx: u32,
) -> VertexOutput {
    var out: VertexOutput;
    let pos_3d = vertex_3d_position_array[in.vertex_id];
    let scale = vec4(view_params.scale, 0.5, 1.0);
    let offset = vec4(view_params.align, 0.5, 0.0);
    out.pos = vec4(pos_3d * scale + offset);
    out.polygon_id = in.polygon_id;
    return out;
}

@fragment
// TODO: consider `@early_depth_test`
fn fs_main(in: VertexOutput) -> @location(0) i32 {
    return in.polygon_id;
}
