@group(0) @binding(0) var blit_texture: texture_2d<f32>;
@group(0) @binding(1) var blit_sampler: sampler;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn blit_vertex(@builtin(vertex_index) idx: u32) -> VertexOutput {
    let x_bit = i32(idx & 1u) != 0;
    let y_bit = i32((idx >> 1u) & 1u) != 0;
    let index = vec2(x_bit, y_bit);

    var out: VertexOutput;
    out.position = vec4(select(vec2(-1.0, -1.0), vec2(1.0, 1.0), index), 0.0, 1.0);
    out.uv = select(vec2(0.0, 1.0), vec2(1.0, 0.0), index);
    return out;
}

@fragment
fn blit_fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(blit_texture, blit_sampler, in.uv);
}
