/*
 * CONSTANTS
 */

// When compiling the shader in Rust, we will fill in the number of dimensions.
const NDIM: i32 = {{ndim}};

// Larger number means less clipping, but also less Z buffer precision.
const Z_CLIP: f32 = 16.0;



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

struct CompositeParams {
    alpha: f32,
    outline_radius: u32,
}

struct SpecialColors {
    background: vec3<f32>,
    outline: vec3<f32>,
}



/*
 * BUFFER BINDINGS
 */

// Static mesh data (per-vertex)
@group(0) @binding(0) var<storage, read> vertex_positions: array<f32>;
@group(0) @binding(1) var<storage, read> u_tangents: array<f32>;
@group(0) @binding(2) var<storage, read> v_tangents: array<f32>;
@group(0) @binding(3) var<storage, read> sticker_shrink_vectors: array<f32>;
@group(0) @binding(4) var<storage, read> piece_ids: array<i32>;
@group(0) @binding(5) var<storage, read> facet_ids: array<i32>;
@group(0) @binding(6) var<storage, read> polygon_ids: array<i32>;

// Static mesh data (other)
@group(1) @binding(0) var<storage, read> piece_centroids: array<f32>;
@group(1) @binding(1) var<storage, read> facet_centroids: array<f32>;
@group(1) @binding(2) var<storage, read> polygon_color_ids: array<i32>;
@group(1) @binding(3) var<storage, read> color_values: array<vec4<f32>>;

// Computed data (per-vertex)
@group(1) @binding(4) var<storage, read_write> vertex_3d_positions: array<vec4<f32>>;
@group(1) @binding(5) var<storage, read_write> vertex_lightings: array<f32>;

// View parameters and transforms
@group(2) @binding(0) var<storage, read> puzzle_transform: array<f32>;
@group(2) @binding(1) var<storage, read> piece_transforms: array<f32>;
@group(2) @binding(2) var<uniform> projection_params: ProjectionParams;
@group(2) @binding(3) var<uniform> lighting_params: LightingParams;
@group(2) @binding(4) var<uniform> view_params: ViewParams;

// Texture samplers
@group(2) @binding(50) var polygon_ids_texture: texture_2d<i32>;

// Composite parameters, which change during a single frame
@group(3) @binding(0)  var<uniform> composite_params: CompositeParams;
@group(3) @binding(1)  var<uniform> special_colors: SpecialColors;



/*
 * SHARED UTILITY FUNCTIONS/STRUCTS
 */

/// Output of `transform_point_to_3d()`.
struct TransformedVertex {
    position: vec4<f32>,
    lighting: f32,
}

/// Transforms a point from NDIM dimensions to 3D.
///
/// Reads from these buffers:
/// - `projection_params`, `lighting_params`, `puzzle_transform`, `piece_transforms`
/// - all static mesh data except `polygon_color_ids` and `color_values`
fn transform_point_to_3d(vertex_index: i32, facet: i32, piece: i32) -> TransformedVertex {
    var ret: TransformedVertex;

    let base_idx = NDIM * vertex_index;

    var new_pos = array<f32, NDIM>();
    var vert_idx = base_idx;
    var facet_centroid_idx = NDIM * facet;
    var piece_centroid_idx = NDIM * piece;
    for (var i = 0; i < NDIM; i++) {
        new_pos[i] = vertex_positions[vert_idx];
        // Apply sticker shrink.
        new_pos[i] += sticker_shrink_vectors[vert_idx] * projection_params.sticker_shrink;
        // Apply facet shrink.
        new_pos[i] -= facet_centroids[facet_centroid_idx];
        new_pos[i] *= 1.0 - projection_params.facet_shrink;
        new_pos[i] += facet_centroids[facet_centroid_idx];
        // Apply piece explode.
        new_pos[i] += piece_centroids[piece_centroid_idx] * projection_params.piece_explode;

        vert_idx++;
        facet_centroid_idx++;
        piece_centroid_idx++;
    }
    var old_pos = new_pos;

    // Apply piece transform.
    new_pos = array<f32, NDIM>();
    var new_u = array<f32, NDIM>();
    var new_v = array<f32, NDIM>();
    vert_idx = base_idx;
    var i: i32 = NDIM * NDIM * piece;
    for (var col = 0; col < NDIM; col++) {
        for (var row = 0; row < NDIM; row++) {
            new_pos[row] += piece_transforms[i] * old_pos[col];
            new_u[row] += piece_transforms[i] * u_tangents[vert_idx];
            new_v[row] += piece_transforms[i] * v_tangents[vert_idx];
            i++;
        }
        vert_idx++;
    }
    old_pos = new_pos;
    var old_u = new_u;
    var old_v = new_v;

    // Apply puzzle transformation and collapse to 4D.
    var point_4d = vec4<f32>();
    var u = vec4<f32>();
    var v = vec4<f32>();
    i = 0;
    for (var col = 0; col < NDIM; col++) {
        // TODO: optimize this
        for (var row = 0; row < NDIM; row++) {
            if (row < NDIM) {
                point_4d[row] += puzzle_transform[i] * old_pos[col];
                u[row] += puzzle_transform[i] * old_u[col];
                v[row] += puzzle_transform[i] * old_v[col];
                i++;
            }
        }
    }

    var w: f32;
    if NDIM < 3 {
        w = 1.0;
    } else {
        w = point_4d.w;
    }

    // Apply 4D perspective transformation.
    let w_divisor = 1.0 / (1.0 + w * projection_params.w_factor_4d);
    let vertex_3d_position = point_4d.xyz * w_divisor;

    // Apply 3D perspective transformation.
    let xy = vertex_3d_position.xy;
    let z = vertex_3d_position.z;
    let z_divisor = 1.0 / (1.0 + (projection_params.fov_signum - z) * projection_params.w_factor_3d);
    let vertex_2d_position = xy * z_divisor;

    // Store the 3D position.
    ret.position = vec4(vertex_2d_position, z, 1.0);

    // Skip lighting computations if possible.
    if lighting_params.directional == 0.0 {
        ret.lighting = lighting_params.ambient;
    } else {
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

        let directional_lighting_amt = dot(normal * orientation, lighting_params.dir) * 0.5 + 0.5;
        ret.lighting = directional_lighting_amt * lighting_params.directional + lighting_params.ambient;
    }

    return ret;
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
    @location(0) lighting: f32,
    @location(1) polygon_id: i32,
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
    return out;
}

@fragment
// TODO: consider `@early_depth_test`
fn render_single_pass_fragment(in: SinglePassVertexOutput) -> @location(0) vec4<f32> {
    var color_id = polygon_color_ids[in.polygon_id];
    color_id = select(color_id + 1, 0, color_id == 65535);
    return vec4(color_values[color_id].rgb * in.lighting, 1.0);
}



/*
 * MAIN PIPELINE
 */

struct PolygonIdsVertexInput {
    @location(0) position: vec4<f32>,
    @location(1) lighting: f32,
    @location(2) polygon_id: i32,
}
struct PolygonIdsVertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) lighting: f32,
    @location(1) polygon_id: i32,
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
    out.polygon_id = in.polygon_id;
    return out;
}

@fragment
// TODO: consider `@early_depth_test`
fn render_polygon_ids_fragment(in: PolygonIdsVertexOutput) -> @location(0) vec2<i32> {
    return vec2(
        // TODO: was previously using red component to store facet ID (for color)
        //       but that's not needed anymore. consider having just a single int
        (i32(in.lighting * 16384.0) << 16u),
        in.polygon_id + 1,
    );
}

struct CompositeVertexInput {
    @location(0) position: vec2<f32>,
    @location(1) uv: vec2<f32>,
}
struct CompositeVertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn render_composite_puzzle_vertex(
    in: CompositeVertexInput,
    @builtin(vertex_index) idx: u32,
) -> CompositeVertexOutput {
    var out: CompositeVertexOutput;
    out.position = vec4<f32>(in.position, 0.0, 1.0);
    out.uv = in.uv;
    return out;
}

@fragment
fn render_composite_puzzle_fragment(in: CompositeVertexOutput) -> @location(0) vec4<f32> {
    let tex_coords: vec2<i32> = vec2<i32>(in.uv * vec2<f32>(textureDimensions(polygon_ids_texture) - vec2(1u, 1u)));

    // TODO: was previously using red component to store facet ID (for color)
    //       but that's not needed anymore. consider having just a single int
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
        return vec4(color_values[polygon_id].rgb * lighting, composite_params.alpha);
    }
}
