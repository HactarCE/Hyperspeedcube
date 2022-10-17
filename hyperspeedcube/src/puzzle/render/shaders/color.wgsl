struct TextureVertex {
    @location(0) pos: vec2<f32>,
    @location(1) uv: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@group(0) @binding(0)
var polygon_ids_texture: texture_2d<i32>;

@group(1) @binding(0)
var color_texture: texture_1d<f32>;

@vertex
fn vs_main(
    in: TextureVertex,
    @builtin(vertex_index) idx: u32,
) -> VertexOutput {
    var out: VertexOutput;
    out.pos = vec4<f32>(in.pos, 0.0, 1.0);
    out.uv = in.uv;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    var uv: vec2<i32> = vec2<i32>(in.uv);
    var polygon_id: i32 = textureLoad(polygon_ids_texture, uv, 0).r;
    var r = 1;
    var a = textureLoad(polygon_ids_texture, uv + vec2(-r, r), 0).r;
    var b = textureLoad(polygon_ids_texture, uv + vec2(-r, -r), 0).r;
    var c = textureLoad(polygon_ids_texture, uv + vec2(r, r), 0).r;
    var d = textureLoad(polygon_ids_texture, uv + vec2(r, -r), 0).r;
    if a != d || b != c {
        return vec4(1.0, 1.0, 1.0, 1.0);
    } else if polygon_id == -1 {
        return vec4(0.0, 0.0, 0.0, 1.0);
    } else {
        return textureLoad(color_texture, polygon_id, 0);
    }
    // return vec4<f32>(f32(polygon_id) / 10.0, 0.1, 0.0, 1.0);
    // return vec4<f32>(vec2<f32>(uv)/1000.0,0.0,1.0);
}
