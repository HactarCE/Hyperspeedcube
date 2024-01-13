/*
 * CONSTANTS
 */

// When compiling the shader in Rust, we will fill in the number of dimensions.
const NDIM: i32 = {{ndim}};

// Larger number means less clipping, but also less Z buffer precision.
const Z_CLIP: f32 = 256.0;

// `w_divisor` below which geometry gets clipped.
const W_DIVISOR_CLIPPING_PLANE: f32 = 0.1;

// Sentinel value indicating no geometry.
const POLYGON_ID_NONE: i32 = -1;

// Color ID for background.
const COLOR_BACKGROUND: u32 = 0x10000u;
// Color ID for outline.
const COLOR_OUTLINE: u32 = 0x10001u;



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
    directional: vec3<f32>,
    ambient: f32,
}

struct ViewParams {
    scale: vec2<f32>,
    align: vec2<f32>,

    clip_4d_backfaces: i32,
    clip_4d_behind_camera: i32,
}

struct CompositeParams {
    outline_radius: u32,
}

struct SpecialColors {
    background: vec3<f32>,
    outline: vec3<f32>,
}



/*
 * BUFFER BINDINGS
 */

// Textures and texture samplers
@group(0) @binding(50)  var sticker_colors: texture_1d<f32>;
@group(0) @binding(51)  var special_colors: texture_1d<f32>;
@group(0) @binding(100) var polygon_ids_texture: texture_2d<u32>;
@group(0) @binding(101) var edges_texture: texture_2d<f32>;
@group(0) @binding(102) var blit_src_texture: texture_2d<f32>;
@group(0) @binding(150) var blit_src_sampler: sampler;

// Static mesh data (per-vertex)
@group(0) @binding(0) var<storage, read> vertex_positions: array<f32>;
@group(0) @binding(1) var<storage, read> u_tangents: array<f32>;
@group(0) @binding(2) var<storage, read> v_tangents: array<f32>;
@group(0) @binding(3) var<storage, read> sticker_shrink_vectors: array<f32>;
@group(0) @binding(4) var<storage, read> piece_ids: array<i32>;
@group(0) @binding(5) var<storage, read> facet_ids: array<i32>;

// Static mesh data (other)
@group(1) @binding(0) var<storage, read> piece_centroids: array<f32>;
@group(1) @binding(1) var<storage, read> facet_centroids: array<f32>;
@group(1) @binding(2) var<storage, read> facet_normals: array<f32>;
@group(1) @binding(3) var<storage, read> polygon_color_ids: array<i32>;

// Computed data (per-vertex)
@group(1) @binding(5) var<storage, read_write> vertex_3d_positions: array<vec4<f32>>;
@group(1) @binding(6) var<storage, read_write> vertex_lightings: array<f32>;
@group(1) @binding(7) var<storage, read_write> vertex_culls: array<f32>;

// View parameters and transforms
@group(2) @binding(0) var<uniform> puzzle_transform: array<vec4<f32>, NDIM>;
@group(2) @binding(1) var<storage, read> piece_transforms: array<f32>;
@group(2) @binding(2) var<storage, read> camera_4d_pos: array<f32, NDIM>;
@group(2) @binding(3) var<uniform> projection_params: ProjectionParams;
@group(2) @binding(4) var<uniform> lighting_params: LightingParams;
@group(2) @binding(5) var<uniform> view_params: ViewParams;

// Composite parameters, which change during a single frame
@group(3) @binding(0) var<uniform> composite_params: CompositeParams;



/*
 * SHARED UTILITY FUNCTIONS/STRUCTS
 */

/// Output of `transform_point_to_3d()`.
struct TransformedVertex {
    position: vec4<f32>,
    lighting: f32,
    cull: i32, // 0 = no cull. 1 = cull.
}

/// Transforms a point from NDIM dimensions to 3D.
///
/// Reads from these buffers:
/// - `projection_params`, `lighting_params`, `puzzle_transform`, `piece_transforms`
/// - all static mesh data except `polygon_color_ids`
fn transform_point_to_3d(vertex_index: i32, facet: i32, piece: i32) -> TransformedVertex {
    var ret: TransformedVertex;
    ret.cull = 0;

    let base_idx = NDIM * vertex_index;

    var new_pos = array<f32, NDIM>();
    var new_normal = array<f32, NDIM>();
    var vert_idx = base_idx;
    var facet_idx = NDIM * facet;
    var piece_idx = NDIM * piece;
    for (var i = 0; i < NDIM; i++) {
        new_pos[i] = vertex_positions[vert_idx];
        new_normal[i] = facet_normals[facet_idx];
        // Apply sticker shrink.
        new_pos[i] += sticker_shrink_vectors[vert_idx] * projection_params.sticker_shrink;
        // Apply facet shrink.
        new_pos[i] -= facet_centroids[facet_idx];
        new_pos[i] *= 1.0 - projection_params.facet_shrink;
        new_pos[i] += facet_centroids[facet_idx];
        // Scale the whole puzzle to compensate for facet shrink.
        new_pos[i] /= 1.0 - projection_params.facet_shrink / 2.0;
        // Apply piece explode.
        new_pos[i] += piece_centroids[piece_idx] * projection_params.piece_explode;
        // Scale back from piece explode.
        new_pos[i] /= 1.0 + projection_params.piece_explode;

        vert_idx++;
        facet_idx++;
        piece_idx++;
    }
    var old_pos = new_pos;
    var old_normal = new_normal;

    // Apply piece transform.
    new_pos = array<f32, NDIM>();
    new_normal = array<f32, NDIM>();
    var new_u = array<f32, NDIM>();
    var new_v = array<f32, NDIM>();
    vert_idx = base_idx;
    var i: i32 = NDIM * NDIM * piece;
    for (var col = 0; col < NDIM; col++) {
        for (var row = 0; row < NDIM; row++) {
            new_pos[row] += piece_transforms[i] * old_pos[col];
            new_normal[row] += piece_transforms[i] * old_normal[col];
            new_u[row] += piece_transforms[i] * u_tangents[vert_idx];
            new_v[row] += piece_transforms[i] * v_tangents[vert_idx];
            i++;
        }
        vert_idx++;
    }
    old_pos = new_pos;
    var old_u = new_u;
    var old_v = new_v;

    // Clip 4D backfaces.
    if NDIM >= 4 && view_params.clip_4d_backfaces != 0 {
        // TODO: these should be `let` bindings. workaround for https://github.com/gfx-rs/wgpu/issues/4920
        var vertex_pos: array<f32, NDIM> = new_pos;
        var vertex_normal: array<f32, NDIM> = new_normal;

        // Compute the dot product `normal · (camera - vertex)`.
        var dot_product_result = 0.0;
        for (var i = 0; i < NDIM; i++) {
            dot_product_result += vertex_normal[i] * (camera_4d_pos[i] - vertex_pos[i]);
        }
        // Cull if the dot product is positive (i.e., the camera is behind the
        // geometry).
        ret.cull |= i32(dot_product_result >= 0.0);
    }

    // Apply puzzle transformation and collapse to 4D.
    var point_4d = vec4<f32>();
    var u = vec4<f32>();
    var v = vec4<f32>();
    i = 0;
    for (var col = 0; col < NDIM; col++) {
        point_4d += puzzle_transform[col] * old_pos[col];
        u += puzzle_transform[col] * old_u[col];
        v += puzzle_transform[col] * old_v[col];
    }

    // Offset the camera to W = -1. Equivalently, move the whole model to be
    // centered on W = 1.
    let w = point_4d.w + 1.0;

    // Apply 4D perspective transformation.
    var w_divisor = 1.0 + w * projection_params.w_factor_4d;
    let vertex_3d_position = point_4d.xyz / w_divisor;
    // Clip geometry that is behind the 4D camera.
    if view_params.clip_4d_behind_camera != 0 {
        ret.cull |= i32(w_divisor < W_DIVISOR_CLIPPING_PLANE);
    }

    // Apply 3D perspective transformation.
    let xy = vertex_3d_position.xy;
    let z = vertex_3d_position.z;
    let z_divisor = 1.0 / (1.0 + (projection_params.fov_signum - z) * projection_params.w_factor_3d);
    let vertex_2d_position = xy * z_divisor;

    // Store the 3D position.
    ret.position = vec4(vertex_2d_position, z, 1.0);

    ret.lighting = lighting_params.ambient;
    // Skip lighting computations if possible.
    let skip_lighting = lighting_params.directional.x == 0.0
                     && lighting_params.directional.y == 0.0
                     && lighting_params.directional.z == 0.0;
    if !skip_lighting {
        // Let:
        //
        //   [ x  y  z  w ] = the initial 4D point
        //   [ x' y' z'   ] = the projected 4D point
        //   α = w_factor_4d
        //
        // We have already computed [x' y' z'] using this formula:
        //
        //   b = 1 / (1 + α w)      = `w_divisor`
        //   x' = x * b
        //
        // Take the Jacobian of this transformation and multiply each tangent
        // vector by it.
        let u_3d = (u.xyz - vertex_3d_position * u.w * projection_params.w_factor_4d) * w_divisor;
        let v_3d = (v.xyz - vertex_3d_position * v.w * projection_params.w_factor_4d) * w_divisor;
        // Do the same thing to project from 3D to 2D.
        let u_2d = (u_3d.xy + vertex_2d_position * u_3d.z * projection_params.w_factor_3d) * z_divisor;
        let v_2d = (v_3d.xy + vertex_2d_position * v_3d.z * projection_params.w_factor_3d) * z_divisor;

        // Use the 3D-perspective-transformed normal to the Z component to
        // figure out which side of the surface is visible.
        let orientation = sign(u_2d.x * v_2d.y - u_2d.y * v_2d.x);
        let normal = normalize(cross(u_3d, v_3d));

        let directional_lighting_amt = dot(normal * orientation, lighting_params.directional) * 0.5 + 0.5;
        ret.lighting += directional_lighting_amt;
    }

    return ret;
}

/// Returns a sticker color or special.
fn get_color(color_id: u32, lighting: f32) -> vec3<f32> {
    let is_special_color = i32(color_id & 0x10000u) != 0;
    return select(
        textureLoad(sticker_colors, color_id, 0).rgb * lighting,
        textureLoad(special_colors, color_id & 0xFFFFu, 0).rgb,
        is_special_color,
    );
}

/// Returns the polygon ID at the given coordinates.
fn load_polygon_id(tex_coords: vec2<i32>) -> i32 {
    return i32(textureLoad(polygon_ids_texture, tex_coords, 0).r & 0x00FFFFFFu) - 1;
}
/// Returns the polygon lighting at the given coordinates.
fn load_polygon_lighting(tex_coords: vec2<i32>) -> f32 {
    return f32(textureLoad(polygon_ids_texture, tex_coords, 0).r >> 24u) / 255.0;
}

/// Converts UV coordinates (0..1) to texture coordinates (0..n-1).
fn uv_to_tex_coords(uv: vec2<f32>, tex_dim: vec2<u32>) -> vec2<i32> {
    return vec2<i32>(uv * vec2<f32>(tex_dim - 1u));
}



/*
 * BLITTING PIPELINE
 */

 struct UvVertexInput {
    @location(0) position: vec2<f32>,
    @location(1) uv: vec2<f32>,
 }

struct UvVertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn uv_vertex(in: UvVertexInput) -> UvVertexOutput {
    var out: UvVertexOutput;
    out.position = vec4(in.position, 0.0, 1.0);
    out.uv = in.uv;
    return out;
}

@fragment
fn blit_fragment(in: UvVertexOutput) -> @location(0) vec4<f32> {
    return textureSample(blit_src_texture, blit_src_sampler, in.uv);
}



/*
 * SINGLE-PASS PIPELINE
 */

struct SinglePassVertexInput {
    @location(0) piece_id: i32,
    @location(1) facet_id: i32,
    @location(2) polygon_id: i32,
}

struct SinglePassVertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) cull: f32, // 0 = no cull. 1 = cull.
    @location(1) lighting: f32,
    @location(2) polygon_id: i32,
}

@vertex
fn render_single_pass_vertex(
    in: SinglePassVertexInput,
    @builtin(vertex_index) idx: u32,
) -> SinglePassVertexOutput {
    let transformed = transform_point_to_3d(i32(idx), in.facet_id, in.piece_id);

    var out: SinglePassVertexOutput;
    let scale = vec4(view_params.scale, 1.0 / Z_CLIP, 1.0);
    let offset = vec4(view_params.align, 0.5, 0.5);
    out.position = vec4(transformed.position * scale + offset);
    out.polygon_id = in.polygon_id;
    out.lighting = clamp(transformed.lighting, 0.0, 1.0);
    out.cull = f32(transformed.cull);
    return out;
}

@fragment
// TODO: consider `@early_depth_test`
fn render_single_pass_fragment(in: SinglePassVertexOutput) -> @location(0) vec4<f32> {
    if in.cull > 0.0 {
        discard;
    }

    let color_id = u32((polygon_color_ids[in.polygon_id] + 1) & 0xFFFF); // wrap max value around to 0
    return vec4(get_color(color_id, in.lighting), 1.0);
}



/*
 * MAIN PIPELINE
 */

struct PolygonIdsVertexInput {
    @location(0) position: vec4<f32>,
    @location(1) cull: f32, // 0 = no cull. 1 = cull.
    @location(2) lighting: f32,
    @location(3) polygon_id: i32,
}
struct PolygonIdsVertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) cull: f32, // 0 = no cull. 1 = cull.
    @location(1) lighting: f32,
    @location(2) polygon_id: i32,
}

@compute
@workgroup_size(256)
fn compute_transform_points(@builtin(global_invocation_id) global_invocation_id: vec3<u32>) {
    let total = i32(arrayLength(&vertex_3d_positions));
    let index = i32(global_invocation_id.x);
    if (index >= total) {
        return;
    }

    let result = transform_point_to_3d(index, facet_ids[index], piece_ids[index]);

    vertex_culls[index] = f32(result.cull);
    vertex_3d_positions[index] = result.position;
    vertex_lightings[index] = result.lighting;
}

@vertex
fn render_polygon_ids_vertex(
    in: PolygonIdsVertexInput,
    @builtin(vertex_index) idx: u32,
) -> PolygonIdsVertexOutput {
    var out: PolygonIdsVertexOutput;
    let scale = vec4(view_params.scale, 1.0 / Z_CLIP, 1.0);
    let offset = vec4(view_params.align, 0.5, 0.5);
    out.position = vec4(in.position * scale + offset);
    out.lighting = clamp(in.lighting, 0.0, 1.0);
    out.polygon_id = in.polygon_id + 1; // +1 because the texture is cleared to 0
    out.cull = f32(in.cull);
    return out;
}

@fragment
// TODO: consider `@early_depth_test`
fn render_polygon_ids_fragment(in: PolygonIdsVertexOutput) -> @location(0) u32 {
    if in.cull > 0.0 {
        discard;
    }

    // Use the top 8 bits for lighting and the bottom 24 bits for polygon ID.
    return (u32(in.lighting * 255.0) << 24u) | u32(in.polygon_id);
}

/// Use with `uv_vertex` as vertex shader.
@fragment
fn render_composite_puzzle_fragment(in: UvVertexOutput) -> @location(0) vec4<f32> {
    let tex_coords: vec2<i32> = uv_to_tex_coords(in.uv, textureDimensions(polygon_ids_texture));

    let lighting: f32 = load_polygon_lighting(tex_coords);
    let polygon_id: i32 = load_polygon_id(tex_coords);
    let r = i32(composite_params.outline_radius);

    // Fetch polygon IDs
    let a = load_polygon_id(tex_coords + vec2(-r, r));
    let b = load_polygon_id(tex_coords + vec2(-r, -r));
    let c = load_polygon_id(tex_coords + vec2(r, r));
    let d = load_polygon_id(tex_coords + vec2(r, -r));
    var color_id: u32;
    if a != d || b != c {
        color_id = COLOR_OUTLINE;
    } else if polygon_id == POLYGON_ID_NONE {
        color_id = COLOR_BACKGROUND;
    } else {
        color_id = u32((polygon_color_ids[polygon_id] + 1) & 0xFFFF); // wrap max value around to 0
    }
    return vec4(get_color(color_id, lighting), 1.0);
}



/*
 * MLAA EDGE DETECTION PIPELINE
 */

/// Use with `uv_vertex` as vertex shader.
@fragment
fn mlaa_edge_detect_fragment(in: UvVertexOutput) -> @location(0) vec4<f32> {
    return vec4(0.0, 0.0, 0.0, 0.0);
}
