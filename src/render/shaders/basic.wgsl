struct RgbaVertex {
    [[location(0)]] pos: vec4<f32>;
    [[location(1)]] normal: vec4<f32>;
    [[location(2)]] color: vec4<f32>;
};

struct VertexOutput {
    [[builtin(position)]] pos: vec4<f32>;
    [[location(0)]] color: vec4<f32>;
};

struct PuzzleUniform {
    transform: mat4x4<f32>;
    light_direction: vec3<f32>;
    min_light: f32;
};

[[group(0), binding(0)]]
var<uniform> u: PuzzleUniform;

[[stage(vertex)]]
fn vs_main(
    v: RgbaVertex,
    [[builtin(vertex_index)]] idx: u32,
) -> VertexOutput {
    var out: VertexOutput;

    // Apply perspective transform.
    out.pos = u.transform * v.pos;

    // Compute lighting here in the vertex shader because all faces are flat.
    var light_amount: f32 = dot(normalize(v.normal.xyz), normalize(u.light_direction));
    var light_multiplier: f32 = mix(u.min_light, 1.0, light_amount/2.0+0.5);
    var rgb = v.color.rgb * light_multiplier;
    out.color = vec4<f32>(rgb, v.color.a);

    return out;
}

[[stage(fragment)]]
fn fs_main(in: VertexOutput) -> [[location(0)]] vec4<f32> {
    return in.color;
}
