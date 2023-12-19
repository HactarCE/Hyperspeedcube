@group(0) @binding(0) var<storage, read> facet_planes: array<vec4<f32>>;
@group(0) @binding(1) var<storage, read> color_values: array<vec4<f32>>;

// View parameters and transforms
@group(2) @binding(0) var<storage, read> puzzle_transform: array<f32>;
@group(2) @binding(1) var<storage, read> piece_transforms: array<f32>;
@group(2) @binding(2) var<uniform> projection_params: ProjectionParams;
@group(2) @binding(3) var<uniform> lighting_params: LightingParams;
@group(2) @binding(4) var<uniform> view_params: ViewParams;

// Texture samplers
@group(2) @binding(50) var polygon_ids_texture: texture_2d<i32>;



/*
 * UNIFORM STRUCTS
 */

struct ProjectionParams {
    facet_shrink: f32,
    sticker_shrink: f32,
    piece_explode: f32,

    w_factor_4d: f32,
    w_factor_3d: f32,
    fov_signum: f32,
}

struct LightingParams {
    dir: vec3<f32>,
    ambient: f32,
    _padding1: vec3<f32>,
    directional: f32,
}

struct ViewParams {
    scale: vec2<f32>,
    align: vec2<f32>,
}



struct RaycastVertexInput {
    @location(0) pos: vec2<f32>,
}

struct RaycastVertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn render_raycast_vertex(
    in: RaycastVertexInput,
) -> RaycastVertexOutput {
    var out: RaycastVertexOutput;
    out.position = vec4(in.pos, 1.0, 1.0);
    let scale = view_params.scale;
    let offset = view_params.align;
    out.uv = in.pos * scale.yx + offset;
    return out;
}

@fragment
fn render_raycast_fragment(in: RaycastVertexOutput) -> @location(0) vec4<f32> {
    var matrix = mat3x3<f32>();
    var i = 0;
    for (var col = 0; col < 3; col++) {
        for (var row = 0; row < 3; row++) {
            matrix[col][row] = puzzle_transform[i];
            i++;
        }
    }

    var min_z = 5.0;
    var draw = false;
    var color = vec4<f32>();
    for (var i = 0; i < 6; i++) {
        var normal = vec3<f32>();
        let distance = 1.0;
        if i == 0 { normal.x =  1.0; }
        if i == 1 { normal.x = -1.0; }
        if i == 2 { normal.y =  1.0; }
        if i == 3 { normal.y = -1.0; }
        if i == 4 { normal.z =  1.0; }
        if i == 5 { normal.z = -1.0; }
        normal = matrix * normal;
        // let normal = matrix * facet_planes[i].xyz;
        // let distance = facet_planes[i].w;
        let z = (distance - dot(normal.xy, in.uv)) / normal.z;
        if z < min_z {
            min_z = z;
            color = color_values[i];
            draw = true;
        }
    }

    if !draw {
        discard;
    }

    return color;
    // return vec4<f32>(in.uv, 0.5, 1.0);
}
