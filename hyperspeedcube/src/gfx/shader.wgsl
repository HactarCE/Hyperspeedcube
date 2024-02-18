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

const OUTLINE_RADIUS: f32 = 0.1;



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
    amt: f32,
}

struct ViewParams {
    scale: vec2<f32>,

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
@group(0) @binding(101) var polygon_ids_depth_texture: texture_depth_2d;
@group(0) @binding(102) var edge_ids_texture: texture_2d<u32>;
@group(0) @binding(103) var edge_ids_depth_texture: texture_depth_2d;
@group(0) @binding(104) var blit_src_texture: texture_2d<f32>;
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
@group(1) @binding(6) var<storage, read_write> vertex_3d_normals: array<vec4<f32>>;
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
    normal: vec3<f32>,
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

    // Store the 3D position, before 3D perspective transformation.
    let z_divisor = z_divisor(vertex_3d_position.z);
    ret.position = vec4(vertex_3d_position, z_divisor);

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
    ret.normal = normalize(cross(u_3d, v_3d)) * orientation;

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
    let xy = pos_3d.xy * view_params.scale;
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

fn transform_depth_to_world_z(depth: f32) -> f32 {
    // TODO: move these to CPU
    let near = Z_CLIP;
    let far = -Z_CLIP;

    // Invert `transform_world_z_to_ndc()`.
    return far + depth * (near - far);
}
fn transform_ndc_to_world_point(ndc: vec3<f32>) -> vec3<f32> {
    let z = transform_depth_to_world_z(ndc.z);
    let xy = ndc.xy / view_params.scale;
    return vec3(xy * z_divisor(z), z);
}

struct Ray {
    origin: vec3<f32>,
    direction: vec3<f32>,
}
fn transform_ndc_to_world_ray(ndc: vec2<f32>) -> Ray {
    let xy = ndc / view_params.scale;
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
        textureLoad(special_colors, color_id & 0xFFFFu, 0).rgb * lighting,
        is_special_color,
    );
}

/// Converts UV coordinates (0..1) to texture coordinates (0..n-1).
fn uv_to_tex_coords(uv: vec2<f32>) -> vec2<i32> {
    return vec2<i32>(uv * target_size);
}

struct RayCapsuleIntersection {
    intersects: i32, // TODO: boolean
    t_ray: f32,
    t_edge: f32,
}

/// Intersect ray with capsule: https://iquilezles.org/articles/intersectors
fn intersect_ray_with_capsule(ray: Ray, pa: vec3<f32>, pb: vec3<f32>, r: f32) -> RayCapsuleIntersection {
    var out: RayCapsuleIntersection;

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
        out.t_edge = saturate(y / baba);
        // body
        if y > 0.0 && y < baba {
            out.intersects = 1;
            out.t_ray = t;
            return out;
        }
        // caps
        let oc: vec3<f32> = select(ro - pb, oa, y <= 0.0);
        b = dot(rd,oc);
        c = dot(oc,oc) - r*r;
        h = b*b - c;
        if h > 0.0 {
            out.intersects = 1;
            out.t_ray = -b - sqrt(h);
            return out;
        }
    }

    out.intersects = 0;
    return out;
}

/// Compute normal vector on surface of capsule:
/// https://www.shadertoy.com/view/Xt3SzX
fn capsule_normal(pos: vec3<f32>, a: vec3<f32>, b: vec3<f32>, r: f32) -> vec3<f32> {
    let ba: vec3<f32> = b - a;
    let pa: vec3<f32> = pos - a;
    let h: f32 = saturate(dot(pa, ba) / dot(ba, ba));
    return (pa - h * ba) / r;
}

fn pack_f32_to_u16(f: f32) -> u32 {
    return u32((saturate(f) + 1.0) * 32767.0);
}
fn unpack_u16_to_f32(u: u32) -> f32 {
    return f32(f32(u) / 32767.0 - 1.0);
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
    out.lighting = 1.0;//saturate(point_3d.lighting);
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
    @location(1) normal: vec4<f32>,
    @location(2) polygon_id: i32,
    @location(3) cull: f32, // 0 = no cull. 1 = cull.
}
struct PolygonIdsVertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) @interpolate(perspective) normal_xy: vec2<f32>,
    @location(1) polygon_id: i32,
    @location(2) cull: f32, // 0 = no cull. 1 = cull.
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

    vertex_3d_positions[index] = point_3d.position;
    vertex_3d_normals[index] = vec4(point_3d.normal, 1.0);
    vertex_culls[index] = f32(point_3d.cull);
}

@vertex
fn render_polygon_ids_vertex(in: PolygonIdsVertexInput) -> PolygonIdsVertexOutput {
    var out: PolygonIdsVertexOutput;
    out.position = transform_world_to_clip_space(in.position);
    out.normal_xy = in.normal.xy;
    out.polygon_id = in.polygon_id + 1; // +1 because the texture is cleared to 0
    out.cull = f32(in.cull);
    return out;
}

@fragment
fn render_polygon_ids_fragment(in: PolygonIdsVertexOutput) -> @location(0) vec2<u32> {
    if in.cull > 0.0 {
        discard;
    }

    // Pack X and Y into a single integer. Use the lower 16 bits for X and the
    // upper 16 bits for Y.
    let packed_normal_xy = pack_f32_to_u16(in.normal_xy.x) | (pack_f32_to_u16(in.normal_xy.y) << 16u);

    return vec2(u32(in.polygon_id), packed_normal_xy);
}



/*
 * FANCY PIPELINE - EDGE IDS
 */

struct EdgeIdsVertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) @interpolate(linear) a: vec4<f32>,
    @location(1) @interpolate(linear) b: vec4<f32>,
    @location(2) @interpolate(linear) xy: vec2<f32>,
    @location(3) @interpolate(linear) t: f32,
    @location(4) @interpolate(linear) u: f32,
    @location(5) @interpolate(linear) cull: f32, // 0 = no cull. 1 = cull.
    @location(6) @interpolate(flat) edge_id: i32,
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

    let world_a = read_vertex_3d_positions[edge_verts.r];
    let world_b = read_vertex_3d_positions[edge_verts.g];

    // Do perspective divide so that we can operate in a sort of screen space
    // with square pixels (as opposed to NDC, where pixels are not square).
    let a = world_a.xy / world_a.w;
    let b = world_b.xy / world_b.w;

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

    // Compute the maximum radius of the outline in NDC.
    let min_z_divisor = min(world_a.w, world_b.w);
    let radius_ndc = OUTLINE_RADIUS / min_z_divisor;

    // Starting with the edge from `a` to `b`, expand it to a rectangle by
    // adding radius `radius_ndc`.
    let vt = b - a;
    var vu = vec2(-vt.y, vt.x); // Rotate 90 degrees CCW
    let extra_radius_as_ratio = radius_ndc / length(vt);

    let t = select(-extra_radius_as_ratio, 1.0 + extra_radius_as_ratio, idx < 2u);
    let u = select(-extra_radius_as_ratio, extra_radius_as_ratio, (idx & 1u) != 0u);

    let pos = a + mat2x2(vt, vu) * vec2(t, u);

    var out: EdgeIdsVertexOutput;
    out.position = vec4(pos * view_params.scale, 0.0, 1.0);
    out.xy = pos;
    out.a = world_a;
    out.b = world_b;
    out.t = t;
    out.u = select(-radius_ndc, radius_ndc, (idx & 1u) != 0u);
    out.cull = read_vertex_culls[edge_verts.r] + read_vertex_culls[edge_verts.g];
    out.edge_id = edge_id + 1; // +1 because the texture is cleared to 0
    return out;
}

/// Computes the T value and depth coordinate of a pixel on an outline.
@fragment
fn render_edge_ids_fragment(in: EdgeIdsVertexOutput) -> EdgeIdsFragmentOutput {
    if in.cull > 0.0 {
        discard;
    }

    // Compute perspective-correct W value: https://www.scratchapixel.com/lessons/3d-basic-rendering/rasterization-practical-implementation/perspective-correct-interpolation-vertex-attributes.html
    let w = 1.0 / (saturate(1.0 - in.t) / in.a.w + saturate(in.t) / in.b.w);

    // Compute radius out from the center, in world coordinates.
    let u = in.u * w;

    // Compute perspective-correct T value. "Flat" here means that we have not
    // yet accounted for the thickness of the capsule.
    let t_flat = in.t * w / in.b.w;

    // Let's define a new orthogonal coordinate system that will be more useful.
    // Its origin is `a` and its three basis vectors are:
    // - vt = b - a
    // - vu = vector perpendicular to b-a and parallel to view plane
    //        (may have any length)
    // - vr = unit vector perpendicular to `t` and `u` vectors
    let vt = in.b.xyz - in.a.xyz;
    let vu = vec3(-vt.y, vt.x, 0.0);
    let vr = normalize(cross(vt, vu));
    if abs(u) > OUTLINE_RADIUS { discard; }
    // Note that we don't really care about the `vu` axis. The camera ray lies
    // within the plane of `vt` and `vr`, and the 3D capsule's intersection with
    // that plane is a 2D capsule with the following radius:
    let r = sqrt(OUTLINE_RADIUS * OUTLINE_RADIUS - u * u);

    // Compute the camera ray.
    // TODO: move some of this computation to CPU
    let p1: vec3<f32> = vec3(in.xy * z_divisor(0.0), 0.0);
    let p2: vec3<f32> = vec3(in.xy * z_divisor(1.0), 1.0);
    let camera_ray_direction = p2 - p1;

    // Intersect the camera ray with the near edge of the outline and compute a
    // new T value.
    let ray_vt = dot(camera_ray_direction, vt) / dot(vt, vt);
    let ray_vr = dot(camera_ray_direction, vr);
    let t = t_flat + ray_vt * r / ray_vr;

    // Now compute distance to a sphere, ignoring perspective distortion.
    let center = in.a.xyz + vt * t;
    let xy_world = in.xy * z_divisor(center.z);
    let xy_delta = xy_world - center.xy;
    let dz_squared = OUTLINE_RADIUS * OUTLINE_RADIUS - dot(xy_delta, xy_delta);
    // if dz_squared <= 0.0 {
    //     discard; // Too far away!
    // }
    let z = center.z + sqrt(saturate(dz_squared));

    var out: EdgeIdsFragmentOutput;
    out.depth = transform_world_z_to_ndc(z);
    // out.color = vec2(u32(in.edge_id), bitcast<u32>((OUTLINE_RADIUS - length(xy_delta))*10.0));
    out.color = vec2(u32(in.edge_id), bitcast<u32>(t - 0.5));
    return out;
}



/*
 * FANCY PIPELINE - COMPOSITING
 */

/// Use with `uv_vertex` as vertex shader.
@fragment
fn render_composite_puzzle_fragment(in: UvVertexOutput) -> @location(0) vec4<f32> {
    let tex_coords: vec2<i32> = uv_to_tex_coords(in.uv);
    let polygon_tex_value: vec2<u32> = textureLoad(polygon_ids_texture, tex_coords, 0).rg;
    let edge_tex_value: vec2<u32> = textureLoad(edge_ids_texture, tex_coords, 0).rg;
    let polygon_depth = textureLoad(polygon_ids_depth_texture, tex_coords, 0);
    let edge_depth = textureLoad(edge_ids_depth_texture, tex_coords, 0);

    let polygon_id = i32(polygon_tex_value.r) - 1;
    let polygon_normal_xy = vec2(
        unpack_u16_to_f32(polygon_tex_value.g & 0x0000FFFFu),
        unpack_u16_to_f32(polygon_tex_value.g >> 16u),
    );
    let polygon_normal = vec3(polygon_normal_xy, sqrt(1.0 - dot(polygon_normal_xy, polygon_normal_xy))); // TODO: this line causes wrong black lighting
    let lighting = mix(1.0, dot(polygon_normal, lighting_params.dir) * 0.5 + 0.5, lighting_params.amt);

    let edge_id = i32(edge_tex_value.r) - 1;

    // TODO: perf of `select()` vs branch
    let color_id: u32 = select(
        COLOR_OUTLINE,
        select(
            // wrap max value around to 0
            u32((polygon_color_ids[polygon_id] + 1) & 0xFFFF),
            COLOR_BACKGROUND,
            polygon_id == NONE,
        ),
        edge_id == NONE || polygon_depth > edge_depth,
    );
    // if edge_id >= 0 {
    //     color_id = edge_color(edge_id)
    // } else if polygon_id == NONE {
    //     color_id = COLOR_BACKGROUND;
    // } else {
    //     color_id = u32((polygon_color_ids[polygon_id] + 1) & 0xFFFF); // wrap max value around to 0
    // }

    let uv = ((vec2<f32>(tex_coords) + vec2(0.5, 0.5)) / vec2<f32>(target_size) * 2.0 - 1.0) * vec2(1.0, -1.0);

    if color_id == COLOR_OUTLINE {
        let t = bitcast<f32>(edge_tex_value.g);
        let xyz = transform_ndc_to_world_point(vec3(uv, edge_depth));
        let a = read_vertex_3d_positions[edge_verts[edge_id].r].xyz;
        let b = read_vertex_3d_positions[edge_verts[edge_id].g].xyz;
        let p = a + (b - a) * t;
        let lighting2 = mix(1.0, dot(normalize(xyz-p), lighting_params.dir) * 0.5 + 0.5, lighting_params.amt);
        // return vec4(get_color(COLOR_OUTLINE, lighting2), 1.0);
        return vec4(0.0-t, t, 0.0, 1.0);
    }

    let base_color = vec4(get_color(color_id, lighting), 1.0);
    return base_color;

    // let uv = ((vec2<f32>(tex_coords) + vec2(0.5, 0.5)) / vec2<f32>(target_size) * 2.0 - 1.0) * vec2(1.0, -1.0);
    // let ray = transform_ndc_to_world_ray(uv);

    // let has_polygon = polygon_id != NONE;
    // let polygon_point = transform_ndc_to_world_point(vec3(uv, polygon_depth));

    // let t = bitcast<f32>(edge_tex_value.g);
    // let edge_verts = edge_verts[edge_id];
    // let a = read_vertex_3d_positions[edge_verts.r].xyz;
    // let b = read_vertex_3d_positions[edge_verts.g].xyz;
    // let edge_point = a + (b - a) * t;
    // let edge_normal = transform_ndc_to_world_point(vec3(uv, edge_depth)) - edge_point;

    // // TODO: consider sampling depth to do accurate blending according to depth
    // // polygon_point.xy = todo!("offset plane points by tex_coords offset");
    // let color_n = edge_color_at_pixel(has_polygon, polygon_point, ray, tex_coords + vec2( 0, -1));
    // let color_s = edge_color_at_pixel(has_polygon, polygon_point, ray, tex_coords + vec2( 0,  1));
    // let color_w = edge_color_at_pixel(has_polygon, polygon_point, ray, tex_coords + vec2(-1,  0));
    // let color_e = edge_color_at_pixel(has_polygon, polygon_point, ray, tex_coords + vec2( 1,  0));
    // let total_alpha: f32 = 1.0 - (1.0 - color_n.a) * (1.0 - color_s.a) * (1.0 - color_w.a) * (1.0 - color_e.a);
    // let blending_weight = select(0.0, total_alpha / (color_n.a + color_s.a + color_w.a + color_e.a), total_alpha > 0.0);
    // let blended_color = (color_n + color_s + color_w + color_e) * blending_weight;
    // return base_color * (1.0 - blended_color.a) + blended_color;
}

fn edge_color_at_pixel(edge_plane: Plane, has_plane: bool, polygon_plane: Plane, tex_coords: vec2<i32>) -> vec4<f32> {
    return vec4<f32>();

    // let edge_id = i32(textureLoad(edge_ids_texture, tex_coords, 0).r) - 1;
    // let z = textureLoad(edge_ids_depth_texture, tex_coords, 0);
    // // TODO: perf of `select()` vs branch
    // return select(
    //     vec4<f32>(),
    //     edge_color_with_alpha(ray, edge_id, edge_plane, has_plane, polygon_plane, z),
    //     is_in_bounds(tex_coords) && edge_id != NONE,
    // );
}

fn is_in_bounds(tex_coords: vec2<i32>) -> bool {
    return 0 <= tex_coords.x
        && 0 <= tex_coords.y
        && tex_coords.x < i32(target_size.x)
        && tex_coords.y < i32(target_size.y);
}

/// Plane in 3D defined by a point and a normal vector. The point is relative to
/// the current pixel.
struct Plane {
    point: vec3<f32>,
    normal: vec3<f32>,
}

fn edge_color_with_alpha(ray: Ray, edge_id: i32, edge_plane: Plane, has_polygon: bool, polygon_plane: Plane, z: f32) -> vec4<f32> {
    let edge_verts = edge_verts[edge_id];
    let a = read_vertex_3d_positions[edge_verts.r].xyz;
    let b = read_vertex_3d_positions[edge_verts.g].xyz;

    // Intersect with a 1-pixel-wider capsule.
    let intersection = intersect_ray_with_capsule(ray, a, b, OUTLINE_RADIUS + 1.0 / (view_params.scale.x * target_size.x) * z_divisor(z));
    let t = intersection.t_ray;
    let intersect_point = (ray.origin + ray.direction * t);

    let plane_point = vec3<f32>(0.0, 0.0, 0.0); // BADDDDD

    var alpha: f32;
    if has_polygon && intersection.intersects != 0 {
        if intersect_point.z < plane_point.z {
            alpha = capsule_alpha_from_radius_vector(plane_plane_intersect_to_line(polygon_plane, edge_plane), z);
            return vec4(1.0, 0.0, 0.0, 1.0);
        } else {
            alpha = 1.0;
            return vec4(1.0, 1.0, 0.0, 1.0);
        }
    } else {
        alpha = capsule_alpha_from_radius_vector(vector_to_capsule_edge(ray, a, b), z);
            // return vec4(0.0, 0.0, 1.0, 1.0);
    }
    // if has_plane &&  {
    //     alpha = 1.0;
    // } else {
    //     alpha = 0.0;
    // }

    // let alpha = select(
    //     capsule_edge_alpha(ray, a, b),
    //     capsule_plane_alpha(ray, a, b, plane_point),
    //     has_plane && t > -Z_CLIP && intersect_point.z < plane_point.z,
    // );

    // TODO: apply lighting
    let lighting = 1.0;

    // TODO: specular highlight?

    // Premultiply alpha.
    let color = vec4(get_color(COLOR_OUTLINE, lighting) * alpha, alpha);
    return color;
}

fn capsule_alpha_from_radius_vector(delta: vec2<f32>, z: f32) -> f32 {
    // Convert to NDC screen space.
    let subpixel_delta = transform_small_world_vector_to_pixel_vector(delta, z);
    // Get the length of `subpixel_delta` in pixel units.
    let subpixel_distance = length(-subpixel_delta); // TODO: handle diagonals better (maybe `max()`?)
    // That subpixel delta is the alpha value for the outline.
    return sqrt(saturate(1.0 - subpixel_distance));
}

fn vector_to_capsule_edge(ray: Ray, a: vec3<f32>, b: vec3<f32>) -> vec2<f32> {
    // Find the closest points on the outline and the ray.
    // https://math.stackexchange.com/a/2217845/1115019
    let perp = cross(b - a, ray.direction);

    // The edge is parametrized from `t=0` at `a` to `t=1` at `b`.
    let t_edge = saturate(dot(cross(ray.direction, perp), ray.origin - a) / dot(perp, perp));
    let pos_edge = a + (b - a) * saturate(t_edge);

    // The ray is parametrized from `t=0` at `ray.origin` to `t=1` at
    // `ray.origin + ray.direction`.
    let t_ray = dot(cross(b - a, perp), ray.origin - a) / dot(perp, perp);
    let pos_ray = ray.origin + ray.direction * t_ray;

    // Compute the vector from the edge to the pixel.
    let vector_from_central_line = pos_ray - pos_edge;

    // Get the component of the vector that is parallel to the screen.
    let xy_delta = vector_from_central_line.xy;
    let xy_delta_unit = normalize(vector_from_central_line).xy;
    // Compute the vector from the pixel to the nearest point on the capsule.
    return xy_delta_unit * OUTLINE_RADIUS - xy_delta;
}

/// Intersects two planes and returns a line (as 2D vector to the nearest
/// point on it and a Z coordinate).
fn plane_plane_intersect_to_line(p1: Plane, p2: Plane) -> vec2<f32> {
    // plane 1: 0 = a1 (x-x1) + b1 (y-y1) + c1 (z-z1)
    // plane 2: 0 = a2 (x-x2) + b2 (y-y2) + c2 (z-z2)
    // intersection (ignoring Z coordinate):
    // 0 = (+ a1 c2
    //      - a2 c1) x
    //   + (+ b1 c2
    //      - b2 c1) y
    //   + (+ a2 x2 c1
    //      + b2 y2 c1
    //      + c2 z2 c1
    //      - a1 x1 c2
    //      - b1 y1 c2
    //      - c1 z1 c2)
    let a = determinant(mat2x2(p1.normal.xz, p2.normal.xz));
    let b = determinant(mat2x2(p1.normal.yz, p2.normal.yz));
    let c = dot(p2.normal, p2.point) * p1.normal.z - dot(p1.normal, p1.point) * p2.normal.z;
    return vec2(c * -0.5) / vec2(a, b);
}
