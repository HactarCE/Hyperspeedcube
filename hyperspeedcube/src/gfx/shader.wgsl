/*
 * CONSTANTS
 */

/// When compiling the shader in Rust, we will fill in the number of dimensions.
const NDIM: i32 = {{ndim}};

/// Near and far plane distance. Larger number means less clipping, but also
/// less Z buffer precision.
const Z_CLIP: f32 = 1024.0;

/// `w_divisor` below which geometry gets clipped.
const W_DIVISOR_CLIPPING_PLANE: f32 = 0.1;

/// Sentinel value indicating no geometry.
const NONE: i32 = -1;

/// Color ID for background.
const COLOR_BACKGROUND: u32 = 0x10000u;
/// Color ID for outline.
const COLOR_OUTLINE: u32 = 0x10001u;

const OUTLINE_RADIUS: f32 = 0.02;



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
@group(0) @binding(100) var ids_texture: texture_2d<u32>;
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
@group(1) @binding(4) var<storage, read> edge_verts: array<vec2<u32>>;
@group(1) @binding(5) var<storage, read> edge_color_ids: array<i32>;

// Computed data (per-vertex)
@group(1) @binding(5) var<storage, read_write> vertex_3d_positions: array<vec4<f32>>;
@group(1) @binding(5) var<storage, read> read_vertex_3d_positions: array<vec4<f32>>;
@group(1) @binding(6) var<storage, read_write> vertex_lightings: array<f32>;
@group(1) @binding(7) var<storage, read_write> vertex_culls: array<f32>; // TODO: pack into bit array
@group(1) @binding(7) var<storage, read> read_vertex_culls: array<f32>; // TODO: pack into bit array

// View parameters and transforms
@group(2) @binding(0) var<uniform> puzzle_transform: array<vec4<f32>, NDIM>;
@group(2) @binding(1) var<storage, read> piece_transforms: array<f32>;
@group(2) @binding(2) var<storage, read> camera_4d_pos: array<f32, NDIM>;
// TODO: consolidate all these
@group(2) @binding(3) var<uniform> projection_params: ProjectionParams;
@group(2) @binding(4) var<uniform> lighting_params: LightingParams;
@group(2) @binding(5) var<uniform> view_params: ViewParams;
@group(2) @binding(6) var<uniform> target_size: vec2<f32>;

// Composite parameters, which change during a single frame
@group(3) @binding(0) var<uniform> composite_params: CompositeParams;



/*
 * SHARED UTILITY FUNCTIONS/STRUCTS
 */

/// Output of `transform_point_to_3d()`.
struct TransformedVertex {
    /// 3D position of the vertex, including W coordinate for
    /// perspective-correct interpolation.
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
    let w_divisor = w_divisor(w);
    let recip_w_divisor = 1.0 / w_divisor;
    let vertex_3d_position = point_4d.xyz * recip_w_divisor;
    // Clip geometry that is behind the 4D camera.
    if view_params.clip_4d_behind_camera != 0 {
        ret.cull |= i32(w_divisor < W_DIVISOR_CLIPPING_PLANE);
    }

    // Store the 3D position.
    let z_divisor = z_divisor(vertex_3d_position.z);
    ret.position = vec4(vertex_3d_position, z_divisor);

    ret.lighting = lighting_params.ambient;
    // Skip lighting computations if possible.
    let skip_lighting = lighting_params.directional.x == 0.0
                     && lighting_params.directional.y == 0.0
                     && lighting_params.directional.z == 0.0;
    if !skip_lighting {
        // Apply 3D perspective transformation.
        let xy = vertex_3d_position.xy;
        let recip_z_divisor = 1.0 / z_divisor;
        var vertex_2d_position = xy * recip_z_divisor;

        // Let:
        //
        //   [ x  y  z  w ] = the initial 4D point
        //   [ x' y' z'   ] = the projected 4D point
        //   α = w_factor_4d
        //
        // We have already computed [x' y' z'] using this formula:
        //
        //   b = 1 / (1 + α w)      = `recip_w_divisor`
        //   x' = x * b
        //
        // Take the Jacobian of this transformation and multiply each tangent
        // vector by it.
        let u_3d = (u.xyz - vertex_3d_position * u.w * projection_params.w_factor_4d) * recip_w_divisor;
        let v_3d = (v.xyz - vertex_3d_position * v.w * projection_params.w_factor_4d) * recip_w_divisor;
        // Do the same thing to project from 3D to 2D.
        let u_2d = (u_3d.xy + vertex_2d_position * u_3d.z * projection_params.w_factor_3d) * recip_z_divisor;
        let v_2d = (v_3d.xy + vertex_2d_position * v_3d.z * projection_params.w_factor_3d) * recip_z_divisor;

        // Use the 3D-perspective-transformed normal to the Z component to
        // figure out which side of the surface is visible.
        var orientation = sign(u_2d.x * v_2d.y - u_2d.y * v_2d.x) * sign(z_divisor);
        let normal = normalize(cross(u_3d, v_3d));

        let directional_lighting_amt = dot(normal * orientation, lighting_params.directional) * 0.5 + 0.5;
        ret.lighting += directional_lighting_amt;
    }

    return ret;
}

/// Returns the XYZ divisor for projection from 4D to 3D, which is based on the
/// W coordinate.
fn w_divisor(w: f32) -> f32 {
    return 1.0 + w * projection_params.w_factor_4d;
}

/// Returns the XY divisor for projection from 3D to 2D, which is based on the Z
/// coordinate.
fn z_divisor(z: f32) -> f32 {
    return 1.0 + (projection_params.fov_signum - z) * projection_params.w_factor_3d;
}

/// Converts a 3D world space position to clip space coordinates. The Z
/// coordinate must be transformed by `depth_value()` before being written to
/// the depth buffer.
fn transform_world_to_clip_space(pos_3d: vec4<f32>) -> vec4<f32> {
    let xy = pos_3d.xy * view_params.scale + view_params.align;
    let z = transform_world_z_to_clip_space(pos_3d.z, pos_3d.w);
    let w = pos_3d.w;

    return vec4(xy, z, w);
}
fn transform_world_z_to_clip_space(z: f32, w: f32) -> f32 {
    // Map [far, near] to [0, 1] after division by W
    return transform_world_z_to_ndc(z) * w;
    // TODO: consider computing fragment depth value separately for better precision
}
fn transform_world_z_to_ndc(z: f32) -> f32 {
    // In thoery we should be able to compute the exact near or far plane
    // depending on the FOV but I couldn't get this to work. There's probably
    // just some silly thing I missed.

    // TODO: see if I can get this code to work

    // // Compute the plane containing the 3D camera; i.e., where the projection
    // // rays converge (which may be behind the puzzle). This gives us either the
    // // near plane or the far plane, depending on the sign of the 3D FOV.
    // //
    // // TODO: move this computation to the CPU
    // let clip_plane = projection_params.fov_signum + 1.0 / projection_params.w_factor_3d;
    // let near = select(Z_CLIP, clip_plane * 1.5, projection_params.w_factor_3d > 1.0 / (Z_CLIP - 1.0));
    // let far = select(-Z_CLIP, clip_plane * 1.5, projection_params.w_factor_3d < -1.0 / (Z_CLIP - 1.0));
    let near = Z_CLIP;
    let far = -Z_CLIP;

    // Map [far, near] to [0, 1]
    return (z - far) / (near - far);
}
fn transform_small_world_vector_to_pixel_vector(v: vec2<f32>, z: f32) -> vec2<f32> {
    return v * view_params.scale * target_size / (2.0 * z_divisor(z));
}

struct Ray {
    origin: vec3<f32>,
    direction: vec3<f32>,
}
fn transform_ndc_to_world_ray(ndc: vec2<f32>) -> Ray {
    let xy = (ndc - view_params.align) / view_params.scale;
    // TODO: move some this computation to CPU
    let p1: vec3<f32> = vec3(xy * z_divisor(1.0), 1.0);
    let p2: vec3<f32> = vec3(xy * z_divisor(-1.0), -1.0);

    var out: Ray;
    out.origin = p1;
    out.direction = normalize(p2 - p1);
    return out;
}

/// Returns a sticker color or special color.
fn get_color(color_id: u32, lighting: f32) -> vec3<f32> {
    let is_special_color = i32(color_id & 0x10000u) != 0;
    return select(
        textureLoad(sticker_colors, color_id, 0).rgb * lighting,
        textureLoad(special_colors, color_id & 0xFFFFu, 0).rgb,
        is_special_color,
    );
}

/// Returns the polygon ID at the given coordinates.
fn get_polygon_id(texture_value: vec2<u32>) -> i32 {
    return i32(texture_value.r & 0x00FFFFFFu) - 1;
}
/// Returns the polygon lighting at the given coordinates.
fn get_polygon_lighting(texture_value: vec2<u32>) -> f32 {
    return f32(texture_value.r >> 24u) / 255.0;
}
/// Returns the edge ID at the given coordinates.
fn get_edge_id(texture_value: vec2<u32>) -> i32 {
    return i32(texture_value.g) - 1;
}

/// Converts UV coordinates (0..1) to texture coordinates (0..n-1).
fn uv_to_tex_coords(uv: vec2<f32>, tex_dim: vec2<u32>) -> vec2<i32> {
    return vec2<i32>(uv * vec2<f32>(tex_dim));
}

/// Intersect ray with capsule: https://iquilezles.org/articles/intersectors
fn intersect_ray_with_capsule(ray: Ray, pa: vec3<f32>, pb: vec3<f32>, r: f32) -> f32 {
    let ro = ray.origin;
    let rd = ray.direction;

    let ba: vec3<f32> = pb - pa;
    let oa: vec3<f32> = ro - pa;

    let baba: f32 = dot(ba, ba);
    let bard: f32 = dot(ba, rd);
    let baoa: f32 = dot(ba, oa);
    let rdoa: f32 = dot(rd, oa);
    let oaoa: f32 = dot(oa, oa);

    let a: f32 = baba      - bard*bard;
    var b: f32 = baba*rdoa - baoa*bard;
    var c: f32 = baba*oaoa - baoa*baoa - r*r*baba;
    var h: f32 = b*b - a*c;
    if h >= 0.0 {
        let t: f32 = (-b-sqrt(h))/a;
        let y: f32 = baoa + t*bard;
        // body
        if y > 0.0 && y < baba {
            return t;
        }
        // caps
        let oc: vec3<f32> = select(ro - pb, oa, y <= 0.0);
        b = dot(rd,oc);
        c = dot(oc,oc) - r*r;
        h = b*b - c;
        if h > 0.0 {
            return -b - sqrt(h);
        }
    }
    return -Z_CLIP;
}

/// Compute normal vector on surface of capsule:
/// https://www.shadertoy.com/view/Xt3SzX
fn capsule_normal(pos: vec3<f32>, a: vec3<f32>, b: vec3<f32>, r: f32) -> vec3<f32> {
    let ba: vec3<f32> = b - a;
    let pa: vec3<f32> = pos - a;
    let h: f32 = clamp(dot(pa, ba) / dot(ba, ba), 0.0, 1.0);
    return (pa - h * ba) / r;
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
    let point_3d = transform_point_to_3d(i32(idx), in.facet_id, in.piece_id);

    var out: SinglePassVertexOutput;
    out.position = transform_world_to_clip_space(point_3d.position);
    out.polygon_id = in.polygon_id;
    out.lighting = clamp(point_3d.lighting, 0.0, 1.0);
    out.cull = f32(point_3d.cull);
    return out;
}

@fragment
fn render_single_pass_fragment(in: SinglePassVertexOutput) -> @location(0) vec4<f32> {
    if in.cull > 0.0 {
        discard;
    }

    let color_id = u32((polygon_color_ids[in.polygon_id] + 1) & 0xFFFF); // wrap max value around to 0
    return vec4(get_color(color_id, in.lighting), 1.0);
}



/*
 * FANCY PIPELINE - POLYGON IDS
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

    let point_3d = transform_point_to_3d(index, facet_ids[index], piece_ids[index]);

    vertex_culls[index] = f32(point_3d.cull);
    vertex_3d_positions[index] = point_3d.position;
    vertex_lightings[index] = clamp(point_3d.lighting, 0.0, 1.0);
}

@vertex
fn render_polygon_ids_vertex(
    in: PolygonIdsVertexInput,
    @builtin(vertex_index) idx: u32,
) -> PolygonIdsVertexOutput {
    var out: PolygonIdsVertexOutput;
    out.position = transform_world_to_clip_space(in.position);
    out.lighting = in.lighting;
    out.polygon_id = in.polygon_id + 1; // +1 because the texture is cleared to 0
    out.cull = f32(in.cull);
    return out;
}

@fragment
fn render_polygon_ids_fragment(in: PolygonIdsVertexOutput) -> @location(0) vec2<u32> {
    if in.cull > 0.0 {
        discard;
    }

    // Use the top 8 bits for lighting and the bottom 24 bits for polygon ID.
    let out = (u32(in.lighting * 255.0) << 24u) | u32(in.polygon_id);
    return vec2(out, 0u);
}



/*
 * FANCY PIPELINE - EDGE IDS
 */

struct EdgeIdsVertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) @interpolate(perspective) xy: vec2<f32>,
    @location(1) @interpolate(flat) a: vec3<f32>,
    @location(3) @interpolate(flat) b: vec3<f32>,
    @location(2) @interpolate(flat) cull: f32, // 0 = no cull. 1 = cull.
    @location(4) @interpolate(flat) edge_id: i32,
}
struct EdgeIdsFragmentOutput {
    @builtin(frag_depth) depth: f32, // written to depth texture
    @location(0) color: vec2<u32>, // written to "color" texture (not actually a color)
}

@vertex
fn render_edge_ids_vertex(
    @location(0) edge_id: i32,
    @builtin(vertex_index) idx: u32,
) -> EdgeIdsVertexOutput {
    let edge_verts = edge_verts[edge_id];
    var a = read_vertex_3d_positions[edge_verts.r];
    var b = read_vertex_3d_positions[edge_verts.g];
    a = a * 0.999 + b * 0.001;
    b = b * 0.999 + a * 0.001;

    // The goal here is to generate a minimal quad that covers the capsule shape
    // of the outline. The correct thing to do here is to compute the
    // perspective-projection of the sphere at either endpoint of the capsule,
    // each of which results in the ellipsoid derived here:
    // https://www.geometrictools.com/Documentation/PerspectiveProjectionEllipsoid.pdf
    //
    // But that's an absolute pain. So instead we'll just approximate it by
    // projecting a billboard of a circular disk. We don't need to be perfectly
    // exact, since we'll trim a pixel or two around the edges in the fragment
    // shader anyway. This approximation is only bad for spheres that are very
    // close to the camera, which is rare and unimportant in this application.
    let min_z_divisor = min(a.w, b.w);
    let max_radius = OUTLINE_RADIUS / min_z_divisor * 10.0;

    let clip_a = transform_world_to_clip_space(a);
    let clip_b = transform_world_to_clip_space(b);
    let ndc_a = clip_a.xy / clip_a.w;
    let ndc_b = clip_b.xy / clip_b.w;
    var u = normalize(ndc_b.xy - ndc_a.xy);
    var v = vec2(-u.y, u.x); // Rotate 90 degrees CCW
    u *= max_radius * view_params.scale;
    v *= max_radius * view_params.scale;
    let base = select(ndc_a, ndc_b, idx % 2u == 1u);
    let offset = select(vec2(-1.0), vec2(1.0), vec2(idx % 2u == 1u, idx == 0u || idx == 1u || idx == 3u));

    var out: EdgeIdsVertexOutput;
    out.xy = base + mat2x2(u, v) * offset;
    out.position = vec4(out.xy, 0.0, 1.0);
    out.a = a.xyz;
    out.b = b.xyz;
    out.cull = read_vertex_culls[edge_verts.r] + read_vertex_culls[edge_verts.g];
    out.edge_id = edge_id;
    return out;
}

@fragment
fn render_edge_ids_fragment(in: EdgeIdsVertexOutput) -> EdgeIdsFragmentOutput {
    if in.cull > 0.0 {
        discard;
    }

    let ray = transform_ndc_to_world_ray(in.xy);
    let t = intersect_ray_with_capsule(ray, in.a, in.b, OUTLINE_RADIUS);
    if t <= -Z_CLIP {
        discard;
    }
    let z = ray.origin.z + ray.direction.z * t;

    var out: EdgeIdsFragmentOutput;
    out.depth = transform_world_z_to_ndc(z);
    out.color = vec2(0u, u32(in.edge_id + 1));
    return out;
}



/*
 * FANCY PIPELINE - COMPOSITING
 */

/// Use with `uv_vertex` as vertex shader.
@fragment
fn render_composite_puzzle_fragment(in: UvVertexOutput) -> @location(0) vec4<f32> {
    let tex_coords: vec2<i32> = uv_to_tex_coords(in.uv, textureDimensions(ids_texture));
    let tex_value: vec2<u32> = textureLoad(ids_texture, tex_coords, 0).rg;

    let lighting: f32 = get_polygon_lighting(tex_value);
    let polygon_id: i32 = get_polygon_id(tex_value);
    let edge_id: i32 = get_edge_id(tex_value);

    // TODO: perf of `select()` vs branch
    let color_id: u32 = select(
        COLOR_OUTLINE,
        select(
            // wrap max value around to 0
            u32((polygon_color_ids[polygon_id] + 1) & 0xFFFF),
            COLOR_BACKGROUND,
            polygon_id == NONE,
        ),
        edge_id == NONE,
    );
    // if edge_id >= 0 {
    //     color_id = edge_color(edge_id)
    // } else if polygon_id == NONE {
    //     color_id = COLOR_BACKGROUND;
    // } else {
    //     color_id = u32((polygon_color_ids[polygon_id] + 1) & 0xFFFF); // wrap max value around to 0
    // }

    let base_color = vec4(get_color(color_id, lighting), 1.0);

    let uv = ((vec2<f32>(tex_coords) + vec2(0.5, 0.5)) / vec2<f32>(textureDimensions(ids_texture)) * 2.0 - 1.0) * vec2(1.0, -1.0);
    let ray = transform_ndc_to_world_ray(uv);

    // TODO: consider sampling depth to do accurate blending according to depth
    let color_n = edge_color_at_pixel(ray, tex_coords + vec2( 0, -1));
    let color_s = edge_color_at_pixel(ray, tex_coords + vec2( 0,  1));
    let color_w = edge_color_at_pixel(ray, tex_coords + vec2(-1,  0));
    let color_e = edge_color_at_pixel(ray, tex_coords + vec2( 1,  0));
    let total_alpha: f32 = 1.0 - (1.0 - color_n.a) * (1.0 - color_s.a) * (1.0 - color_w.a) * (1.0 - color_e.a);
    let blending_weight = select(0.0, total_alpha / (color_n.a + color_s.a + color_w.a + color_e.a), total_alpha > 0.0);
    let blended_color = (color_n + color_s + color_w + color_e) * blending_weight;
    return base_color * (1.0 - blended_color.a) + blended_color;
}

fn edge_color_at_pixel(ray: Ray, tex_coords: vec2<i32>) -> vec4<f32> {
    let edge_id = get_edge_id(textureLoad(ids_texture, tex_coords, 0).rg);
    // TODO: perf of `select()` vs branch
    return select(
        vec4<f32>(),
        edge_color_with_alpha(ray, edge_id),
        is_in_bounds(tex_coords) && edge_id != NONE,
    );
}

fn is_in_bounds(tex_coords: vec2<i32>) -> bool {
    return 0 <= tex_coords.x
        && 0 <= tex_coords.y
        && tex_coords.x < i32(textureDimensions(ids_texture).x)
        && tex_coords.y < i32(textureDimensions(ids_texture).y);
}

fn edge_color_with_alpha(ray: Ray, edge_id: i32) -> vec4<f32> {
    let edge_verts = edge_verts[edge_id];
    let a = read_vertex_3d_positions[edge_verts.r].xyz;
    let b = read_vertex_3d_positions[edge_verts.g].xyz;

    // Find the closest points on the outline and the ray.
    // https://math.stackexchange.com/a/2217845/1115019
    let perp = cross(b - a, ray.direction);

    // The edge is parametrized from `t=0` at `a` to `t=1` at `b`.
    let t_edge = clamp(dot(cross(ray.direction, perp), ray.origin - a) / dot(perp, perp), 0.0, 1.0);
    let pos_edge = a + (b - a) * clamp(t_edge, 0.0, 1.0);

    // The ray is parametrized from `t=0` at `ray.origin` to `t=1` at
    // `ray.origin + ray.direction`.
    let t_ray = dot(cross(b - a, perp), ray.origin - a) / dot(perp, perp);
    let pos_ray = ray.origin + ray.direction * t_ray;

    // Compute the vector from the edge to the pixel.
    let delta_from_edge = pos_ray - pos_edge;
    // Get the component of the vector that is parallel to the screen.
    let xy_delta = delta_from_edge.xy;
    let xy_delta_unit = normalize(delta_from_edge).xy;
    // Compute the vector from the nearest point on the capsule to the pixel.
    let delta_from_capsule = xy_delta - xy_delta_unit * OUTLINE_RADIUS;
    // Convert to NDC screen space.
    let subpixel_delta_from_capsule = transform_small_world_vector_to_pixel_vector(delta_from_capsule, pos_ray.z);
    // Get the length of `subpixel_delta_from_capsule` in pixel units.
    let subpixel_distance = length(subpixel_delta_from_capsule);
    // That subpixel delta is the alpha value for the outline.
    let alpha = sqrt(clamp(1.0 - subpixel_distance, 0.0, 1.0));

    // TODO: apply lighting
    let lighting = 1.0;

    // TODO: specular highlight?

    // Premultiply alpha.
    let color = vec4(get_color(COLOR_OUTLINE, lighting) * alpha, alpha);
    return color;
}
