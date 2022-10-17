struct PolygonVertex {
    @location(0) pos: vec3<f32>,
    @location(1) polygon_id: i32,
}

struct VertexOutput {
    @builtin(position) pos: vec4<f32>,
    @location(0) polygon_id: i32,
}

struct BasicUniform {
    scale: vec2<f32>,
    align: vec2<f32>,
}

@group(0) @binding(0)
var<uniform> u: BasicUniform;

@vertex
fn vs_main(
    in: PolygonVertex,
    @builtin(vertex_index) idx: u32,
) -> VertexOutput {
    var out: VertexOutput;
    out.pos = vec4<f32>(in.pos.xy * u.scale + u.align, in.pos.z * 0.5 + 0.5, 1.0);
    out.polygon_id = in.polygon_id;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) i32 {
    return in.polygon_id;
}
