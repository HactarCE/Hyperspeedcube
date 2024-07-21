/*
 * CONSTANTS
 */

/// When compiling the shader in Rust, we will fill in the number of dimensions.
const NDIM: i32 = {{ndim}};

/// `w_divisor` below which geometry gets clipped.
const W_DIVISOR_CLIPPING_PLANE: f32 = 0.1;

/// Color ID for background.
const COLOR_BACKGROUND: u32 = 0u;



/*
 * UNIFORM STRUCTS
 */

struct PrecomputedValues {
    /// Near plane Z coordinate.
    n: f32,
    /// Far plane Z coordinate.
    f: f32,

    /// Near plane Z divisor: `z_divisor(near_plane_z)`
    npzd: f32,
    /// Far plane Z divisor: `z_divisor(far_plane_z)`
    fpzd: f32,
    /// Z divisor at Z=0: `z_divisor(0.0)`
    z0zd: f32,

    /// `z_divisor(near_plane_z) * z_divisor(far_plane_z)`
    npzd_fpzd: f32,

    /// `w_factor_3d * fov_signum + 1.0`
    wf_s_plus_1: f32,

    /// `n * z_divisor(far_plane_z)`
    n_fpzd: f32,
    /// `w_factor_3d * z_divisor(far_plane_z)`
    wf_fpzd: f32,

    /// `n - f`
    nf: f32,
    /// `(n - f) * wf`
    nf_wf: f32,
    /// `(n - f) * z_divisor(0.0)`
    nf_z0zd: f32,
}

struct DrawParams {
    // Precomputed values for functions
    pre: PrecomputedValues,

    // Lighting
    light_dir: vec3<f32>,
    face_light_intensity: f32,
    outline_light_intensity: f32,

    // Rendering
    pixel_size: f32,
    target_size: vec2<f32>,
    xy_scale: vec2<f32>,

    // Cursor state
    cursor_pos: vec2<f32>,

    // Geometry
    facet_scale: f32,
    gizmo_scale: f32,
    sticker_shrink: f32,
    piece_explode: f32,

    // Projection
    w_factor_4d: f32,
    w_factor_3d: f32,
    fov_signum: f32,
    show_frontfaces: i32,
    show_backfaces: i32,
    clip_4d_behind_camera: i32,
    camera_4d_w: f32,

    first_gizmo_vertex_index: i32,
}



/*
 * BUFFER BINDINGS
 */

// Textures and texture samplers
@group(0) @binding(50)  var color_palette_texture: texture_1d<f32>;
@group(0) @binding(100) var polygons_texture: texture_2d<u32>;
@group(0) @binding(101) var polygons_depth_texture: texture_depth_2d;
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
@group(0) @binding(5) var<storage, read> surface_ids: array<i32>;

// Static mesh data (other)
@group(1) @binding(0) var<storage, read> piece_centroids: array<f32>;
@group(1) @binding(1) var<storage, read> surface_centroids: array<f32>;
@group(1) @binding(2) var<storage, read> surface_normals: array<f32>;
@group(1) @binding(3) var<storage, read> edge_verts: array<vec2<u32>>;
// Computed data (per-vertex)
@group(1) @binding(4) var<storage, read_write> vertex_3d_positions: array<vec4<f32>>;
@group(1) @binding(4) var<storage, read> read_vertex_3d_positions: array<vec4<f32>>;
@group(1) @binding(5) var<storage, read_write> vertex_3d_normals: array<vec4<f32>>;

// View parameters and transforms
@group(2) @binding(0) var<uniform> puzzle_transform: array<vec4<f32>, NDIM>;
@group(2) @binding(1) var<storage, read> piece_transforms: array<f32>;
@group(2) @binding(2) var<storage, read> camera_4d_pos: array<f32, NDIM>; // storage instead of uniform because it's not padded to a multiple of 16 bytes
@group(2) @binding(3) var<storage, read> polygon_color_ids: array<u32>;
@group(2) @binding(4) var<storage, read> outline_color_ids: array<u32>;
@group(2) @binding(5) var<storage, read> outline_radii: array<f32>;
@group(2) @binding(6) var<uniform> draw_params: DrawParams;



/*
 * SHARED UTILITY FUNCTIONS/STRUCTS
 */

/// Output of `transform_point_to_3d()`.
struct TransformedVertex {
    /// 3D position of the vertex, including W coordinate for
    /// perspective-correct interpolation.
    position: vec4<f32>,
    normal: vec3<f32>,
    cull: bool,
}

/// Transforms a point from NDIM dimensions to 3D.
///
/// Reads from these buffers:
/// - `puzzle_transform`, `piece_transforms`, `draw_params`
/// - all static mesh data except `polygon_color_ids`
fn transform_point_to_3d(vertex_index: i32, surface: i32, piece: i32) -> TransformedVertex {
    var ret: TransformedVertex;
    ret.cull = false;

    let is_puzzle_vertex: bool = vertex_index < draw_params.first_gizmo_vertex_index;

    let base_idx = NDIM * vertex_index;

    var new_pos = array<f32, NDIM>();
    var new_normal = array<f32, NDIM>();
    var vert_idx = base_idx;
    var surface_idx = NDIM * surface;
    var piece_idx = NDIM * piece;
    for (var i = 0; i < NDIM; i++) {
        new_pos[i] = vertex_positions[vert_idx];
        new_normal[i] = surface_normals[surface_idx];
        if is_puzzle_vertex {
            // Apply sticker shrink.
            new_pos[i] += sticker_shrink_vectors[vert_idx] * draw_params.sticker_shrink;
        }
        // Apply facet shrink.
        new_pos[i] -= surface_centroids[surface_idx];
        if is_puzzle_vertex {
            new_pos[i] *= draw_params.facet_scale;
        } else {
            new_pos[i] *= draw_params.gizmo_scale;
        }
        new_pos[i] += surface_centroids[surface_idx];
        if is_puzzle_vertex {
            // Apply piece explode.
            new_pos[i] += piece_centroids[piece_idx] * draw_params.piece_explode;
        }

        vert_idx++;
        surface_idx++;
        piece_idx++;
    }
    var old_pos = new_pos;
    var old_normal = new_normal;
    var old_u: array<f32, NDIM>;
    var old_v: array<f32, NDIM>;
    var i: i32;
    if is_puzzle_vertex {
        // Apply piece transform.
        new_pos = array<f32, NDIM>();
        new_normal = array<f32, NDIM>();
        var new_u = array<f32, NDIM>();
        var new_v = array<f32, NDIM>();
        vert_idx = base_idx;
        i = NDIM * NDIM * piece;
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
        old_normal = new_normal;
        old_u = new_u;
        old_v = new_v;
    }

    // Apply puzzle transformation and collapse to 4D.
    var point_4d = vec4<f32>();
    var normal_4d = vec4<f32>();
    var u = vec4<f32>();
    var v = vec4<f32>();
    i = 0;
    for (var col = 0; col < NDIM; col++) {
        point_4d += puzzle_transform[col] * old_pos[col];
        normal_4d += puzzle_transform[col] * old_normal[col];
        u += puzzle_transform[col] * old_u[col];
        v += puzzle_transform[col] * old_v[col];
    }

    // Clip 4D backfaces.
    if NDIM >= 4 {
        // Is the camera behind or in front of the geometry? Compute the dot
        // product `normal · (vertex - camera)`.
        let camera_ray_4d = point_4d - vec4(0.0, 0.0, 0.0, draw_params.camera_4d_w);
        var dot_product_result = dot(normal_4d, camera_ray_4d);
        // Add extra dimensions into the dot product.
        for (var i = 4; i < NDIM; i++) {
            // The puzzle transform doesn't apply to dimensions higher than 4D.
            dot_product_result += old_normal[i] * old_pos[i];
        }
        let show = dot_product_result == 0.0
            || (dot_product_result > 0.0 && draw_params.show_frontfaces != 0)
            || (dot_product_result < 0.0 && draw_params.show_backfaces != 0);
        ret.cull |= !show;
    }

    // Apply 4D perspective transformation.
    let w_divisor = w_divisor(point_4d.w);
    let recip_w_divisor = 1.0 / w_divisor;
    let vertex_3d_position = point_4d.xyz * recip_w_divisor;
    // Clip geometry that is behind the 4D camera.
    if draw_params.clip_4d_behind_camera != 0 {
        ret.cull |= w_divisor < W_DIVISOR_CLIPPING_PLANE;
    }

    // Store the 3D position, before 3D perspective transformation.
    let z_divisor = z_divisor(vertex_3d_position.z);
    ret.position = vec4(vertex_3d_position, z_divisor);

    // Apply 3D perspective transformation.
    let xy = vertex_3d_position.xy;
    let recip_z_divisor = 1.0 / z_divisor;
    let vertex_2d_position = xy * recip_z_divisor;

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
    let u_3d = (u.xyz + vertex_3d_position * u.w * draw_params.w_factor_4d) * recip_w_divisor;
    let v_3d = (v.xyz + vertex_3d_position * v.w * draw_params.w_factor_4d) * recip_w_divisor;
    // Do the same thing to project from 3D to 2D.
    let u_2d = (u_3d.xy + vertex_2d_position * u_3d.z * draw_params.w_factor_3d) * recip_z_divisor;
    let v_2d = (v_3d.xy + vertex_2d_position * v_3d.z * draw_params.w_factor_3d) * recip_z_divisor;

    // Use the 3D-perspective-transformed normal to the Z component to
    // figure out which side of the surface is visible.
    let orientation = sign(u_2d.x * v_2d.y - u_2d.y * v_2d.x) * sign(z_divisor);
    ret.normal = normalize(cross(u_3d, v_3d)) * orientation;

    return ret;
}

/// Returns the XYZ divisor for projection from 4D to 3D, which is based on the
/// W coordinate.
fn w_divisor(w: f32) -> f32 {
    return 1.0 + (1.0 - w) * draw_params.w_factor_4d;
}

/// Returns the XY divisor for projection from 3D to 2D, which is based on the Z
/// coordinate.
fn z_divisor(z: f32) -> f32 {
    // 1.0 + (draw_params.fov_signum - z) * draw_params.w_factor_3d
    // = (wf * s + 1) - (wf * z)
    let pre = draw_params.pre;
    return pre.wf_s_plus_1 - draw_params.w_factor_3d * z;
}

/// Converts a 3D world space Z coordinate to a value written directly to the
/// depth buffer.
fn transform_world_z_to_depth(z: f32) -> f32 {
    // Map [near, far] to [0, 1] in the shape of a reciprocal function with an
    // asymptote at the camera Z coordinate.
    //
    // Given these constants:
    //
    //    n  = near_plane_z
    //    f  = far_plane_z
    //    wf = w_factor_3d
    //    s  = fov_signum
    //
    // Here's a Desmos plot showing this function and its inverse:
    // https://www.desmos.com/calculator/micuh2ldlw
    //
    // ( n - z )( wf*(s-f) + 1 )
    // -------------------------
    // ( n - f )( wf*(s-z) + 1 )
    //
    // We can rearrange this formula into a form that is easy to compute with
    // respect to z:
    //
    // z * -(wf*(s-f)+1) + n*(wf*(s-f)+1)
    // ---------------------------------
    // z *    (n-f)*wf   + (n-f)*(wf*s+1)
    //
    // Some of those values are things we have names for:
    //
    // n-f                       = nf
    // wf*(s-f)+1 = z_divisor(f) = fpzd
    // wf*s+1     = z_divisor(0) = z0zd
    //
    // Using those values precomputed on the CPU, we get this compact form:
    //
    // z * -fpzd + n_fpzd
    // -------------------
    // z * nf_wf + nf_z0zd
    let pre = draw_params.pre;
    return (z * -pre.fpzd + pre.n_fpzd) / (z * -pre.nf_wf + pre.nf_z0zd);
}
/// Converts a depth value to a 3D world space Z coordinate.
fn transform_depth_to_world_z(depth: f32) -> f32 {
    // Map [1, 0] to [far, near] in the shape of a reciprocal function with an
    // asymptote at the camera's depth coordinate.
    //
    // See `transform_world_z_to_depth()` for the function we want to invert
    // (and the corresponding Desmos plot). Suffice to say, this is the function
    // we want to implement (where d=depth):
    //
    // d * -(n-f)*(wf*s+1) + n*(wf*(s-f)+1)
    // ------------------------------------
    // d *   (wf*(f-n))    +   wf*(s-f)+1
    //
    // Some of those values are things we have names for:
    //
    // n-f                       = nf
    // wf*(s-f)+1 = z_divisor(f) = fpzd
    // wf*s+1     = z_divisor(0) = z0zd
    //
    // Using those values precomputed on the CPU, we get this compact form:
    //
    //  d * -nf_z0zd + n_fpzd
    // ----------------------
    //    d * nf_wf + fpzd
    let pre = draw_params.pre;
    return (depth * -pre.nf_z0zd + pre.n_fpzd) / (depth * -pre.nf_wf + pre.fpzd);
}

/// Converts a 3D world space Z coordinate to the NDC Z coordinate.
fn transform_world_z_to_clip_space(z: f32, w: f32) -> f32 {
    // Map [far, near] to [0, 1] after division by W
    return transform_world_z_to_depth(z) * w;
}

/// Converts a 3D world space position to clip space coordinates.
fn transform_world_to_clip_space(pos_3d: vec4<f32>) -> vec4<f32> {
    let xy = pos_3d.xy * draw_params.xy_scale;
    let z = transform_world_z_to_clip_space(pos_3d.z, pos_3d.w);
    let w = pos_3d.w;

    return vec4(xy, z, w);
}

/// Unprojects a "screen space" position to a world space position.
///
/// "Screen space" is like world coordinates, but only at the perspective-fixed
/// plane (either Z=+1.0 or Z=-1.0, depending on FOV sign) where `z_divisor=1`.
fn transform_screen_space_to_world_point(screen_space_xy: vec2<f32>, depth: f32) -> vec3<f32> {
    let z = transform_depth_to_world_z(depth);
    return vec3(screen_space_xy * z_divisor(z), z);
}
/// Projects a point in world space to a point in screen space.
fn project_world_point_to_screen_space(pos_3d: vec3<f32>) -> vec2<f32> {
    let recip_z_divisor = 1.0 / z_divisor(pos_3d.z);
    return pos_3d.xy * recip_z_divisor;
}
/// Projects a vector in world space to a vector in screen space.
///
/// Internally, this uses the Jacobian of the perspective projection
/// transformation at the given world point.
fn project_world_vector_to_screen_space(pos_3d: vec3<f32>, vector: vec3<f32>) -> vec2<f32> {
    let recip_z_divisor = 1.0 / z_divisor(pos_3d.z);
    return (vector.xy + pos_3d.xy * vector.z * draw_params.w_factor_3d * recip_z_divisor) * recip_z_divisor;
}

/// Ray in 3D space.
struct Ray {
    /// Point somewhere along the ray.
    origin: vec3<f32>,
    /// Normalized direction vector.
    direction: vec3<f32>,
}
/// Returns a ray that crosses the perspective-fixed plane (either Z=+1.0 or
/// Z=-1.0, depending on FOV sign) at the given XY coordinates.
fn transform_screen_space_to_world_ray(screen_space_xy: vec2<f32>) -> Ray {
    var out: Ray;
    out.origin = vec3(screen_space_xy, draw_params.fov_signum);
    out.direction = normalize(vec3(screen_space_xy * draw_params.w_factor_3d, -1.0));
    return out;
}

/// Returns a color by ID, with premultiplied alpha.
fn get_color(color_id: u32, lighting: f32) -> vec4<f32> {
    var light_value = lighting;

    // Override light_value if highest bit is set.
    if (color_id & 0x80000000u) != 0u {
        light_value = 1.0;
    }

    let index = color_id & 0x7FFFFFFFu;

    let color = textureLoad(color_palette_texture, index, 0);
    // Premultiply alpha.
    return vec4(color.rgb * light_value, 1.0) * color.a;
}
/// Returns the lighting multiplier, given a normal vector.
fn compute_lighting(normal: vec3<f32>, intensity: f32) -> f32 {
    return mix(1.0, dot(normal, draw_params.light_dir) * 0.5 + 0.5, intensity);
}

/// Converts UV coordinates (0..1) to texture coordinates (0..n-1).
fn uv_to_tex_coords(uv: vec2<f32>) -> vec2<i32> {
    return vec2<i32>(uv * draw_params.target_size);
}

struct RayCapsuleIntersection {
    intersects: bool,
    t_ray: f32,
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
        let t: f32 = (-b - sqrt(h))/a;
        let y: f32 = baoa + t*bard;
        // body
        if y > 0.0 && y < baba {
            out.intersects = true;
            out.t_ray = t;
            return out;
        }
        // caps
        let oc: vec3<f32> = select(ro - pb, oa, y <= 0.0);
        b = dot(rd,oc);
        c = dot(oc,oc) - r*r;
        h = b*b - c;
        if h > 0.0 {
            out.intersects = true;
            out.t_ray = -b - sqrt(h);
            return out;
        }
    }

    out.intersects = false;
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

/// Packs a normal vector (assumed to be facing the camera) into a 4-byte
/// integer. Only the X and Y of the normal vector are necessary.
fn pack_normal_vector_to_u32(xy: vec2<f32>) -> u32 {
    // Pack X and Y into a single integer. Use the lower 16 bits for X and the
    // upper 16 bits for Y.
    return pack_f32_to_u16(xy.x) | (pack_f32_to_u16(xy.y) << 16u);
}
/// Unpacks a normal vector (assumed to be facing the camera) from a 4-byte
/// integer.
fn unpack_normal_vector_from_u32(u: u32) -> vec3<f32> {
    let xy = vec2(
        unpack_f32_from_u16(u & 0x0000FFFFu),
        unpack_f32_from_u16(u >> 16u),
    );
    // The `saturate()` here is necessary to guard against slight numerical
    // errors producing a negative number inside the square root.
    return vec3(xy, sqrt(saturate(1.0 - dot(xy, xy))));
}

fn pack_f32_to_u16(f: f32) -> u32 {
    // Map [-1.0, 1.0] to [0, 65535]
    return u32(saturate(f * 0.5 + 0.5) * 65535.0);
}
fn unpack_f32_from_u16(u: u32) -> f32 {
    // Map [0, 65535] to [-1.0, 1.0]
    return (f32(u) / 65535.0) * 2.0 - 1.0;
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
    return linear_to_gamma(textureSample(blit_src_texture, blit_src_sampler, in.uv));
}



/*
 * RENDER POLYGONS
 */

struct PolygonsVertexInput {
    @location(0) position: vec4<f32>,
    @location(1) normal: vec4<f32>,
    @location(2) polygon_id: i32,
}
struct PolygonsVertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) @interpolate(perspective) normal_xy: vec2<f32>,
    @location(1) color_id: u32,
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

    var point_3d = transform_point_to_3d(index, surface_ids[index], piece_ids[index]);
    // Indicate that the vertex should be culled by setting W=0.
    point_3d.position.w = select(point_3d.position.w, 0.0, point_3d.cull);

    vertex_3d_positions[index] = point_3d.position;
    vertex_3d_normals[index] = vec4(point_3d.normal, 1.0);
}

@vertex
fn render_polygons_vertex(in: PolygonsVertexInput) -> PolygonsVertexOutput {
    var out: PolygonsVertexOutput;
    out.position = transform_world_to_clip_space(in.position);
    out.normal_xy = in.normal.xy;
    out.color_id = polygon_color_ids[in.polygon_id];
    out.cull = f32(in.position.w == 0.0);
    return out;
}

@fragment
fn render_polygons_fragment(in: PolygonsVertexOutput) -> @location(0) vec2<u32> {
    if in.cull > 0.0 {
        discard;
    }

    return vec2(u32(in.color_id), pack_normal_vector_to_u32(in.normal_xy));
}



/*
 * RENDER EDGES
 */

struct EdgeIdsVertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) @interpolate(flat) a: vec4<f32>,
    @location(1) @interpolate(flat) b: vec4<f32>,
    @location(2) @interpolate(linear) screen_space_xy: vec2<f32>,
    @location(3) @interpolate(flat) cull: f32, // 0 = no cull. 1 = cull.
    @location(4) @interpolate(flat) edge_id: i32,
    @location(5) @interpolate(flat) radius: f32,
}
struct EdgeIdsFragmentOutput {
    @builtin(frag_depth) depth: f32, // written to depth texture
    @location(0) edge_id: u32, // written to "color" texture (not actually a color)
}

@vertex
fn render_edge_ids_vertex(
    @location(0) edge_id: i32,
    @builtin(vertex_index) idx: u32,
) -> EdgeIdsVertexOutput {
    var capsule_radius = outline_radii[edge_id];
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
    // close to the camera, which is rare and unimportant in this application,
    // and in that case it'll overestimate.

    // Compute the minimum radius of the outline in NDC.
    let min_z = clamp(
        min(world_a.z, world_b.z) - capsule_radius,
        draw_params.pre.f, // far plane Z
        draw_params.pre.n, // near plane Z
    );
    let max_z_divisor = z_divisor(min_z);
    let min_radius = capsule_radius / max_z_divisor;
    // Ensure the outline radius is at least 2 pixels wide.
    capsule_radius *= max(1.0, draw_params.pixel_size / min_radius);

    // Compute the maximum radius of the outline in NDC.
    let max_z = clamp(
        max(world_a.z, world_b.z),
        draw_params.pre.f, // far plane Z
        draw_params.pre.n, // near plane Z
    );
    let min_z_divisor = z_divisor(max_z);
    let radius = capsule_radius / min_z_divisor;

    // Starting with the edge from `a` to `b`, expand it to a rectangle by
    // adding radius `radius`.
    let vt = b - a;
    let vu = vec2(-vt.y, vt.x); // Rotate 90 degrees CCW
    let extra_radius_as_ratio = radius / length(vt);

    let t = select(-extra_radius_as_ratio, 1.0 + extra_radius_as_ratio, idx < 2u);
    let u = select(-extra_radius_as_ratio, extra_radius_as_ratio, (idx & 1u) != 0u);

    let screen_space_xy = a + mat2x2(vt, vu) * vec2(t, u);

    var out: EdgeIdsVertexOutput;
    out.position = vec4(screen_space_xy * draw_params.xy_scale, 0.0, 1.0);
    out.a = world_a;
    out.b = world_b;
    out.screen_space_xy = screen_space_xy;
    out.cull = f32((world_a.w == 0.0) | (world_b.w == 0.0));
    out.edge_id = edge_id + 1; // +1 because the texture is cleared to 0
    out.radius = capsule_radius;
    return out;
}

/// Computes the T value and depth coordinate of a pixel on an outline.
@fragment
fn render_edge_ids_fragment(in: EdgeIdsVertexOutput) -> EdgeIdsFragmentOutput {
    if in.cull > 0.0 {
        discard;
    }

    let ray = transform_screen_space_to_world_ray(in.screen_space_xy);
    let intersection = intersect_ray_with_capsule(ray, in.a.xyz, in.b.xyz, in.radius);
    if !intersection.intersects {
        discard;
    }
    let z = ray.origin.z + ray.direction.z * intersection.t_ray;

    var out: EdgeIdsFragmentOutput;
    out.depth = transform_world_z_to_depth(z);
    out.edge_id = u32(in.edge_id);
    return out;
}



/*
 * COMPOSITE POLYGONS & EDGES
 */

/// Use with `uv_vertex` as vertex shader.
@fragment
fn render_composite_puzzle_fragment(in: UvVertexOutput) -> @location(0) vec4<f32> {
    let tex_coords: vec2<i32> = uv_to_tex_coords(in.uv);
    let screen_space = (in.uv * 2.0 - 1.0) * vec2(1.0, -1.0) / draw_params.xy_scale;
    let ray = transform_screen_space_to_world_ray(screen_space);

    // Compute the polygon plane.
    let polygon = get_polygon_pixel(screen_space, tex_coords);

    // For each edge (center, N, S, W, E), compute its color.
    let center = get_edge_pixel(ray, polygon, screen_space, tex_coords);
    // TODO(optimization): ignore edge pixels that have the same ID as center
    let n = get_edge_pixel(ray, polygon, screen_space, tex_coords + vec2( 0, -1));
    let s = get_edge_pixel(ray, polygon, screen_space, tex_coords + vec2( 0,  1));
    let w = get_edge_pixel(ray, polygon, screen_space, tex_coords + vec2(-1,  0));
    let e = get_edge_pixel(ray, polygon, screen_space, tex_coords + vec2( 1,  0));

    let depths = array(center.depth, n.depth, s.depth, w.depth, e.depth);
    let colors = array(center.color, n.color, s.color, w.color, e.color);

    // Execute the optimal sorting network on 5 elements.
    var a: DepthsAndColors;
    a.depths = depths;
    a.colors = colors;
    a = compare_and_swap(a, 0, 3);
    a = compare_and_swap(a, 1, 4);
    a = compare_and_swap(a, 0, 2);
    a = compare_and_swap(a, 1, 3);
    a = compare_and_swap(a, 0, 1);
    a = compare_and_swap(a, 2, 4);
    a = compare_and_swap(a, 1, 2);
    a = compare_and_swap(a, 3, 4);
    a = compare_and_swap(a, 2, 3);

    var composited_edge_color = a.colors[0];
    for (var i = 1; i < 5; i++) {
        composited_edge_color = composited_edge_color * (1.0 - a.colors[i].a) + a.colors[i];
    }

    return polygon.color * (1.0 - composited_edge_color.a) + composited_edge_color;
}

struct DepthsAndColors {
    depths: array<f32, 5>,
    colors: array<vec4<f32>, 5>,
}
fn compare_and_swap(a: DepthsAndColors, i: i32, j: i32) -> DepthsAndColors {
    var in = a; // TODO: this line shouldn't be necessary. workaround for https://github.com/gfx-rs/wgpu/issues/4920
    let swap = in.depths[i] < in.depths[j];

    var out = in;
    out.depths[i] = select(in.depths[i], in.depths[j], swap);
    out.depths[j] = select(in.depths[i], in.depths[j], !swap);
    out.colors[i] = select(in.colors[i], in.colors[j], swap);
    out.colors[j] = select(in.colors[i], in.colors[j], !swap);
    return out;
}

struct PolygonPixel {
    /// Perspective-projected plane of the polygon surface.
    plane: Plane,
    color: vec4<f32>,
    point: vec3<f32>,
    depth: f32,
}
fn get_polygon_pixel(screen_space: vec2<f32>, tex_coords: vec2<i32>) -> PolygonPixel {
    var out: PolygonPixel;

    out.depth = textureLoad(polygons_depth_texture, tex_coords, 0);
    out.point = transform_screen_space_to_world_point(screen_space, out.depth);

    let tex_value: vec2<u32> = textureLoad(polygons_texture, tex_coords, 0).rg;
    let color_id = tex_value.r;
    if color_id == COLOR_BACKGROUND { // TODO: is this special case beneficial?
        out.plane.v = vec4(0.0, 0.0, 1.0, draw_params.pre.f);
        out.color = get_color(COLOR_BACKGROUND, 1.0);
        out.point = vec3(screen_space * z_divisor(draw_params.pre.f), draw_params.pre.f);
        return out;
    }
    let polygon_normal = unpack_normal_vector_from_u32(tex_value.g);

    out.plane = perspective_project_plane(
        plane_from_point_and_normal(out.point, polygon_normal)
    );

    let lighting = compute_lighting(polygon_normal, draw_params.face_light_intensity);
    out.color = get_color(color_id, lighting);

    return out;
}

struct EdgePixel {
    color: vec4<f32>,
    depth: f32,
}
fn get_edge_pixel(ray: Ray, polygon: PolygonPixel, screen_space: vec2<f32>, tex_coords: vec2<i32>) -> EdgePixel {
    var out: EdgePixel;

    let tex_value: u32 = select(
        0u,
        textureLoad(edge_ids_texture, tex_coords, 0).r,
        is_in_bounds(tex_coords),
    );
    let edge_id = i32(tex_value) - 1;
    if edge_id == -1 {
        out.color = vec4<f32>();
        out.depth = draw_params.pre.f;
        return out;
    }

    let edge_depth: f32 = textureLoad(edge_ids_depth_texture, tex_coords, 0);

    let capsule_radius = outline_radii[edge_id];
    let edge_verts = edge_verts[edge_id];
    let a = read_vertex_3d_positions[edge_verts.r].xyz;
    let b = read_vertex_3d_positions[edge_verts.g].xyz;
    let vt = b - a;

    // Compute the nearest point on the line segment of the capsule.
    let point_on_surface = transform_screen_space_to_world_point(screen_space, edge_depth);
    let t = saturate(dot(point_on_surface - a, vt) / dot(vt, vt));
    let point_on_line_segment = a + vt * t;

    // There's two possible cases for anti-aliasing here: either the capsule is
    // not occluded by a polygon, or it is occluded by a polygon.
    let is_polygon_in_front = polygon.point.z > point_on_line_segment.z;
    // Either way, we'll find a 2D line that locally exactly bounds the visible
    // part of the capsule.
    var edge_line: Line2D;

    if is_polygon_in_front {
        // If the polygon is in front of that plane, then the capsule may be
        // occluded by the polygon. In that case, compute the point on the line
        // segment that is closest to the polygon.
        let new_t = saturate(dot(polygon.point - a, vt) / dot(vt, vt));
        let point_on_line_segment_near_polygon = a + vt * new_t;

        // We get the polygon plane for free. The intersection of the polygon
        // plane with the surface of the capsule gives the outline of the
        // visible portion of the capsule.
        let normal = normalize(polygon.point - point_on_line_segment_near_polygon);
        let point_on_surface_near_polygon = point_on_line_segment_near_polygon + normal * capsule_radius;
        let plane_tangent_to_capsule = perspective_project_plane(
            plane_from_point_and_normal(point_on_surface_near_polygon, normal)
        );

        // Intersect the polygon plane with the plane tangent to the capsule to get a line.
        edge_line = plane_plane_intersect_to_line(polygon.plane, plane_tangent_to_capsule);
    }

    // Find the closest points on the outline and the ray.
    // https://math.stackexchange.com/a/2217845/1115019
    let perp = cross(vt, ray.direction);

    // The edge is parametrized from `t=0` at `a` to `t=1` at `b`.
    let t_edge = saturate(dot(cross(ray.direction, perp), ray.origin - a) / dot(perp, perp));
    let pos_edge = a + vt * t_edge;

    // The ray is parametrized from `t=0` at `ray.origin` to `t=1` at
    // `ray.origin + ray.direction`.
    let t_ray = dot(pos_edge - ray.origin, ray.direction) / dot(ray.direction, ray.direction);
    // let t_ray = dot(cross(vt, perp), ray.origin - a) / dot(perp, perp);
    let pos_ray = ray.origin + ray.direction * t_ray;

    // Compute the vector from the edge to the pixel, and use that to
    // compute the line on the edge of the polygon.
    let delta_from_edge = pos_ray - pos_edge;
    let n = normalize(delta_from_edge) * capsule_radius;
    if !is_polygon_in_front {
        edge_line = line_from_point_and_normal(
            project_world_point_to_screen_space(pos_edge + n),
            project_world_vector_to_screen_space(pos_edge + n, n),
        );
    }

    // Compute how much of the pixel is on one side of the line. This is the
    // "coverage" value, which we'll use to determine alpha.
    let rect_size = vec2(4.0 * draw_params.pixel_size);
    var edge_coverage = screen_space_line_rect_area(edge_line, screen_space, rect_size);

    // Subtract the amount from the other side of the line. This only has an
    // effect for very thin outlines.
    let far_edge_line = line_from_point_and_normal(
        project_world_point_to_screen_space(pos_edge - n),
        project_world_vector_to_screen_space(pos_edge - n, -n)
    );
    edge_coverage -= 1.0 - screen_space_line_rect_area(far_edge_line, screen_space, rect_size);

    // Compute lighting.
    let lighting = compute_lighting(normalize(point_on_surface - point_on_line_segment), draw_params.outline_light_intensity);

    out.color = get_color(outline_color_ids[edge_id], lighting) * saturate(edge_coverage);
    out.depth = edge_depth;

    return out;
}

/// Returns whether `tex_coords` is within bounds of the target.
fn is_in_bounds(tex_coords: vec2<i32>) -> bool {
    return all((vec2(0) <= tex_coords) & (tex_coords < vec2<i32>(draw_params.target_size)));
}

/// Line in 2D represented using the equation v.xy ⋅ p = v.z, where `p` is an
/// arbitrary point.
struct Line2D {
    v: vec3<f32>,
}

/// Constructs a 2D line from a point and a normal vector. The normal vector
/// does not need to be normalized.
fn line_from_point_and_normal(point: vec2<f32>, normal: vec2<f32>) -> Line2D {
    var out: Line2D;
    out.v = vec3(normal, dot(point, normal));
    return out;
}

/// Computes the portion of a rectangle that is on one side of a line.
fn screen_space_line_rect_area(line: Line2D, rect_center: vec2<f32>, rect_size: vec2<f32>) -> f32 {
    // Offset and scale line the line so that the rectangle is actually the unit
    // square centered at the origin.
    var ab = line.v.xy * rect_size.xy;
    let c = line.v.z - dot(line.v.xy, rect_center);

    // Reflect so that `ab` is in the positive quadrant.
    ab = abs(ab);
    // Reflect diagonally so that `a > b`.
    let a = max(ab.x, ab.y);
    let b = min(ab.x, ab.y);

    // Invert so that `c > 0`.
    let abs_c = abs(c);
    let sign_c = sign(c);

    // Now the area above the line is either a trapezoid, a triangle, or nothing.
    let is_trapezoid = a - b - 2.0 * abs_c > 0.0;
    let is_nothing = dot(ab, vec2(0.5, 0.5)) < abs_c;
    // If it's a trapezoid, it has width=1 and height=1/2-c/a.
    let trapezoid_area = 0.5 - abs_c/a;
    // If it's a triangle, its vertices are (0.5, 0.5), (0.5, (c-a/2)/b),
    // ((c-b/2)/a, 0.5). Rearranging the area of that, we get this:
    let tmp = a + b - 2.0 * abs_c;
    let triangle_area = tmp*tmp / (8.0 * a * b);

    let result = saturate(select(select(triangle_area, trapezoid_area, is_trapezoid), 0.0, is_nothing));

    // Flip the area depending on the sign of `c`.
    return select(result, 1.0 - result, c > 0.0);
}

/// Plane in 3D represented using the equation v.xyz ⋅ p = v.w, where `p` is an
/// arbitrary point.
struct Plane {
    v: vec4<f32>,
}

/// Returns the `t` value along the ray where it intersects a plane.
fn intersect_ray_with_plane(ray: Ray, plane: Plane) -> f32 {
    // v.xyz . (ro + rd * t) = v.w
    // v.xyz . ro + v.xyz . rd * t = v.w
    // v.xyz . rd * t = v.w - v.xyz . ro
    // t = (v.w - v.xyz . ro) / (v.xyz . rd)
    return (plane.v.w - dot(ray.origin, plane.v.xyz)) / dot(ray.direction, plane.v.xyz);
}

/// Constructs a plane from a point and a normal vector. The normal vector does
/// not need to be normalized.
fn plane_from_point_and_normal(point: vec3<f32>, normal: vec3<f32>) -> Plane {
    var out: Plane;
    out.v = vec4(normal, dot(point, normal));
    return out;
}

/// Applies the perspective projection transformation to a plane.
fn perspective_project_plane(p: Plane) -> Plane {
    // This is REAL nasty.
    //
    // Given these constants:
    //
    //    n  = near_plane_z
    //    f  = far_plane_z
    //    wf = w_factor_3d
    //    s  = fov_signum
    //
    // And this initial plane equation:
    //
    //    ax + by + cz = d
    //
    // Our goal is to write a new plane equation in terms of:
    //
    //    x' = x / z_divisor(z); y' = y / z_divisor(z); z' =
    //    transform_world_z_to_depth(z);
    //
    // Of course, those values are expressed in terms of z, whereas we want them
    // in terms of z'. Luckily we have `transform_depth_to_world_z()` which
    // gives us z in terms of z'. Substituting that into the equation gives us a
    // really awful plane equation with a z' in the denominator, but we can
    // scale the whole equation by that denominator and then combine like terms
    // to get this new plane equation:
    //
    //    x' * a * z_divisor(n) * z_divisor(f)
    //  + y' * b * z_divisor(n) * z_divisor(f)
    //  + z' * ( c * -(n-f) * z_divisor(0) + d * (n-f) * wf )
    //  =      + c *    - n * z_divisor(f) + d * z_divisor(f)
    //
    // Luckily most of those are pure constants, so we can precompute them on
    // the CPU:
    let pre = draw_params.pre;
    let ab = p.v.xy;
    let cd = p.v.zw;
    var out: Plane;
    out.v = vec4(
        ab * pre.npzd_fpzd,
        dot(cd, vec2(-pre.nf_z0zd, pre.nf_wf)),
        dot(cd, vec2(-pre.n_fpzd, pre.fpzd)),
    );
    return out;
}

/// Intersects two planes in 3D space and returns a line projected into 2D
/// screen space.
fn plane_plane_intersect_to_line(p1: Plane, p2: Plane) -> Line2D {
    // plane 1: 0 = a1 x + b1 y + c1 z - d1
    // plane 2: 0 = a2 x + b2 y + c2 z - d2
    //
    // Intersection (ignoring Z coordinate):
    //
    // 0 = (+ a1 c2
    //      - a2 c1) x
    //   + (+ b1 c2
    //      - b2 c1) y
    //   - (+ d1 c2
    //      - d2 c1)
    let a = determinant(mat2x2(p1.v.xz, p2.v.xz));
    let b = determinant(mat2x2(p1.v.yz, p2.v.yz));
    let c = determinant(mat2x2(p1.v.wz, p2.v.wz));
    var out: Line2D;
    out.v = vec3(a, b, c);
    return out;
}



// Copyright 2019 Google LLC.
// SPDX-License-Identifier: Apache-2.0

// Polynomial approximation in GLSL for the Turbo colormap
// Original LUT: https://gist.github.com/mikhailov-work/ee72ba4191942acecc03fe6da94fc73f

// Authors:
//   Colormap Design: Anton Mikhailov (mikhailov@google.com)
//   GLSL Approximation: Ruofei Du (ruofei@google.com)
//   WGSL Port: Andrew Farkas

fn turbo(value: f32, min: f32, max: f32) -> vec4<f32> {
    let kRedVec4: vec4<f32> = vec4(0.13572138, 4.61539260, -42.66032258, 132.13108234);
    let kGreenVec4: vec4<f32> = vec4(0.09140261, 2.19418839, 4.84296658, -14.18503333);
    let kBlueVec4: vec4<f32> = vec4(0.10667330, 12.64194608, -60.58204836, 110.36276771);
    let kRedVec2: vec2<f32> = vec2(-152.94239396, 59.28637943);
    let kGreenVec2: vec2<f32> = vec2(4.27729857, 2.82956604);
    let kBlueVec2: vec2<f32> = vec2(-89.90310912, 27.34824973);

    let x = saturate((value - min) / (max - min));
    if abs(x) < 0.51 && abs(x) > 0.49 {
        return vec4(1.0, 1.0, 1.0, 1.0);
    }
    let v4: vec4<f32> = vec4( 1.0, x, x * x, x * x * x);
    let v2: vec2<f32> = v4.zw * v4.z;
    return vec4(
        dot(v4, kRedVec4)   + dot(v2, kRedVec2),
        dot(v4, kGreenVec4) + dot(v2, kGreenVec2),
        dot(v4, kBlueVec4)  + dot(v2, kBlueVec2),
        1.0,
    );
}

fn linear_to_gamma(linear: vec4<f32>) -> vec4<f32> {
    // from http://chilliant.blogspot.com/2012/08/srgb-approximations-for-hlsl.html
    return max(1.055 * pow(linear, vec4(0.416666667)) - 0.055, vec4(0.0));
}
