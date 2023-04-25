struct CompositeVertex {
    @location(0) position: vec2<f32>,
    @location(1) uv: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

struct SpecialColors {
    background: vec3<f32>,
    outline: vec3<f32>,
}
struct CompositeParams {
    alpha: f32,
    outline_radius: u32,
}

@group(0) @binding(0) var<uniform> composite_params: CompositeParams;
@group(0) @binding(1) var<uniform> special_colors: SpecialColors;

@group(1) @binding(0) var polygon_ids_texture: texture_2d<i32>;

@group(2) @binding(0) var<storage> facet_colors: array<vec4<f32>>;

@vertex
fn vs_main(
    in: CompositeVertex,
    @builtin(vertex_index) idx: u32,
) -> VertexOutput {
    var out: VertexOutput;
    out.position = vec4<f32>(in.position, 0.0, 1.0);
    out.uv = in.uv;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let tex_coords: vec2<i32> = vec2<i32>(in.uv * vec2<f32>(textureDimensions(polygon_ids_texture) - vec2(1, 1)));

    let facet_id: i32 = textureLoad(polygon_ids_texture, tex_coords, 0).r & 0xFFFF;
    let lighting: f32 = f32(textureLoad(polygon_ids_texture, tex_coords, 0).r >> 16u) / 16384.0;
    let polygon_id: i32 = textureLoad(polygon_ids_texture, tex_coords, 0).g;
    let r = i32(composite_params.outline_radius);

    // Fetch polygon IDs
    let a = textureLoad(polygon_ids_texture, tex_coords + vec2(-r, r), 0).g;
    let b = textureLoad(polygon_ids_texture, tex_coords + vec2(-r, -r), 0).g;
    let c = textureLoad(polygon_ids_texture, tex_coords + vec2(r, r), 0).g;
    let d = textureLoad(polygon_ids_texture, tex_coords + vec2(r, -r), 0).g;
    if a != d || b != c {
        return vec4(special_colors.outline, composite_params.alpha);
    } else if polygon_id == 0 {
        return vec4(special_colors.background, composite_params.alpha);
    } else {
        return vec4(facet_colors[facet_id].rgb * lighting, composite_params.alpha);
    }
}
