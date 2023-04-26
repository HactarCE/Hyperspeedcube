struct PolygonVertex {
    @location(0) position: vec4<f32>,
    @location(1) lighting: f32,
    @location(2) facet_id: i32,
    @location(3) polygon_id: i32,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) lighting: f32,
    @location(1) facet_id: i32,
    @location(2) polygon_id: i32,
}

struct ViewParams {
    scale: vec2<f32>,
    align: vec2<f32>,
}

// Larger number means less clipping, but also less Z buffer precision.
const Z_CLIP: f32 = 16.0;

@group(0) @binding(0) var<uniform> view_params: ViewParams;

@vertex
fn vs_main(
    in: PolygonVertex,
    @builtin(vertex_index) idx: u32,
) -> VertexOutput {
    var out: VertexOutput;
    let scale = vec4(view_params.scale, 1.0 / Z_CLIP, 1.0);
    let offset = vec4(view_params.align, 0.5, 0.5);
    out.position = vec4(in.position * scale + offset);
    out.lighting = clamp(in.lighting, 0.0, 1.0);
    out.facet_id = in.facet_id + 1;
    out.polygon_id = in.polygon_id;
    return out;
}

@fragment
// TODO: consider `@early_depth_test`
fn fs_main(in: VertexOutput) -> @location(0) vec2<i32> {
    return vec2(
        (i32(in.lighting * 16384.0) << 16u) | in.facet_id,
        in.polygon_id + 1,
    );
}
