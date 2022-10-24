struct CompositeVertex {
    @location(0) pos: vec2<f32>,
    @location(1) uv: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

struct CompositeParams {
    background_color: vec3<f32>,
    alpha: f32,
    outline_color: vec3<f32>,
    outline_radius: u32,
}

@group(0) @binding(0) var<uniform> composite_params: CompositeParams;

@group(1) @binding(0) var<storage, read> polygon_color_array: array<vec4<f32>>;

@group(2) @binding(0) var polygon_ids_texture: texture_2d<i32>;

@vertex
fn vs_main(
    in: CompositeVertex,
    @builtin(vertex_index) idx: u32,
) -> VertexOutput {
    var out: VertexOutput;
    out.pos = vec4<f32>(in.pos, 0.0, 1.0);
    out.uv = in.uv;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    var tex_coords: vec2<i32> = vec2<i32>(in.uv * vec2<f32>(textureDimensions(polygon_ids_texture) - vec2(1, 1)));
    var polygon_id: i32 = textureLoad(polygon_ids_texture, tex_coords, 0).x;
    var r = i32(composite_params.outline_radius);
    var a = textureLoad(polygon_ids_texture, tex_coords + vec2(-r, r), 0).r;
    var b = textureLoad(polygon_ids_texture, tex_coords + vec2(-r, -r), 0).r;
    var c = textureLoad(polygon_ids_texture, tex_coords + vec2(r, r), 0).r;
    var d = textureLoad(polygon_ids_texture, tex_coords + vec2(r, -r), 0).r;
    if a != d || b != c {
        return vec4(composite_params.outline_color, composite_params.alpha);
    } else if polygon_id == -1 {
        return vec4(composite_params.background_color, composite_params.alpha);
    } else {
        return vec4(polygon_color_array[polygon_id].rgb, composite_params.alpha);
    }
}
