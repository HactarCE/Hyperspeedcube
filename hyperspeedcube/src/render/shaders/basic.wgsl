struct RgbaVertex {
    @location(0) pos: vec3<f32>,
    @location(1) color: vec4<f32>,
}

struct VertexOutput {
    @builtin(position) pos: vec4<f32>,
    @location(0) color: vec4<f32>,
}

struct BasicUniform {
    scale: vec2<f32>,
    align: vec2<f32>,
}

@group(0) @binding(0)
var<uniform> u: BasicUniform;

@vertex
fn vs_main(
    in: RgbaVertex,
    @builtin(vertex_index) idx: u32,
) -> VertexOutput {
    var out: VertexOutput;
    out.pos = vec4<f32>(in.pos.xy * u.scale + u.align, in.pos.z, 1.0);
    out.color = in.color;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return in.color;
}
