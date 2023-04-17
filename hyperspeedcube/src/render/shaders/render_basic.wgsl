struct Vertex {
    @location(0) pos: vec3<f32>,
    @location(1) color: vec3<f32>,
}

struct VertexOutput {
    @builtin(position) pos: vec4<f32>,
    @location(0) color: vec3<f32>,
}

struct ViewParams {
    @location(0) view_matrix: mat4x4<f32>,
}

@group(0) @binding(0) var<uniform> view_params: ViewParams;

@vertex
fn vs_main(
    in: Vertex,
    @builtin(vertex_index) idx: u32,
) -> VertexOutput {
    var out: VertexOutput;
    out.pos = view_params.view_matrix * vec4<f32>(in.pos, 1.0);
    out.pos.z = (out.pos.z + 1.0) * 0.5;
    out.color = in.color;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(in.color, 1.0);
}
