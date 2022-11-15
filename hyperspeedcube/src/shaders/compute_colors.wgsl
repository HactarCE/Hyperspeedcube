struct PolygonInfo {
    facet: u32,
    v0: u32,
    v1: u32,
    v2: u32,
};

struct LightingParams {
    dir: vec3<f32>,
    ambient: f32,
    directional: f32,
}

@group(0) @binding(0) var<uniform> lighting_params: LightingParams;

@group(1) @binding(0) var<storage, read> polygon_info_array: array<PolygonInfo>;
@group(1) @binding(1) var<storage, read_write> polygon_color_array: array<vec4<f32>>;

@group(1) @binding(2) var<storage, read> vertex_3d_position_array: array<vec4<f32>>;

@group(2) @binding(0) var facet_colors_texture: texture_1d<f32>;

// When compiling the shader in Rust, we will fill in the workgroup size.
@compute
@workgroup_size({{workgroup_size}})
fn main(@builtin(global_invocation_id) global_invocation_id: vec3<u32>) {
    let total = arrayLength(&polygon_color_array);
    let index = global_invocation_id.x;
    if (index >= total) {
        return;
    }

    let polygon_info: PolygonInfo = polygon_info_array[index];

    // Fetch the user-defined facet color.
    let facet_color = textureLoad(facet_colors_texture, i32(polygon_info.facet), 0);

    // Fetch three vertices in the polygon.
    let v0 = vertex_3d_position_array[polygon_info.v0].xyz;
    let v1 = vertex_3d_position_array[polygon_info.v1].xyz;
    let v2 = vertex_3d_position_array[polygon_info.v2].xyz;
    // Compute the polygon's normal vector using a cross product.
    let normal = normalize(cross(v1 - v0, v2 - v0));

    // Calculate brightness.
    let directional_brightness = dot(normal, lighting_params.dir) * 0.5 + 0.5;
    let brightness = lighting_params.ambient + lighting_params.directional * directional_brightness;

    // Write the polygon color.
    polygon_color_array[index] = facet_color * brightness;
}
