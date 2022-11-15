struct StickerInfo {
    piece: u32,
    facet: u32,
};

struct ProjectionParams {
    facet_scale: f32,
    sticker_scale: f32,
    w_factor_4d: f32,
    w_factor_3d: f32,
    fov_signum: f32,
}

// When compiling the shader in Rust, we will fill in the number of dimensions.
let NDIM = {{ndim}}u;

@group(0) @binding(0) var<uniform> projection_params: ProjectionParams;

@group(1) @binding(0) var<storage, read> puzzle_transform: array<f32>;

@group(1) @binding(1) var<storage, read> piece_transform_array: array<f32>;

@group(1) @binding(2) var<storage, read> facet_shrink_center_array: array<f32>;

@group(1) @binding(3) var<storage, read> sticker_info_array: array<StickerInfo>;

@group(1) @binding(4) var<storage, read> vertex_sticker_id_data: array<u32>;
@group(1) @binding(5) var<storage, read> vertex_position_array: array<f32>;
@group(1) @binding(6) var<storage, read> vertex_shrink_vector_array: array<f32>;
@group(1) @binding(7) var<storage, read_write> vertex_3d_position_array: array<vec4<f32>>;

// When compiling the shader in Rust, we will fill in the workgroup size.
@compute
@workgroup_size({{workgroup_size}})
fn main(@builtin(global_invocation_id) global_invocation_id: vec3<u32>) {
    let total = arrayLength(&vertex_3d_position_array);
    let index = global_invocation_id.x;
    if (index >= total) {
        return;
    }

    let sticker: u32 = vertex_sticker_id_data[index];
    let piece: u32 = sticker_info_array[sticker].piece;
    let facet: u32 = sticker_info_array[sticker].facet;

    // Apply sticker scaling.
    var initial = array<f32, NDIM>();
    var vert_idx = NDIM * index;
    for (var i = 0u; i < NDIM; i++) {
        initial[i] = vertex_position_array[vert_idx];
        initial[i] += vertex_shrink_vector_array[vert_idx] * (1.0 - projection_params.sticker_scale);
        vert_idx++;
    }

    // Apply facet scaling.
    var j = facet * NDIM;
    for (var i = 0u; i < NDIM; i++) {
        initial[i] -= facet_shrink_center_array[j];
        initial[i] *= projection_params.facet_scale;
        initial[i] += facet_shrink_center_array[j];
        j++;
    }

    // Apply piece transformation.
    var old_pos = initial;
    var new_pos = array<f32, NDIM>();
    var i: u32 = NDIM * NDIM * piece;
    for (var col = 0u; col < NDIM; col++) {
        for (var row = 0u; row < NDIM; row++) {
            new_pos[row] += piece_transform_array[i] * old_pos[col];
            i++;
        }
    }
    old_pos = new_pos;

    // Apply puzzle transformation and collapse to 4D.
    var point_4d = vec4<f32>();
    var i = 0;
    for (var col = 0u; col < NDIM; col++) {
        // TODO: optimize this
        for (var row = 0u; row < NDIM; row++) {
            if (row < NDIM) {
                point_4d[row] += puzzle_transform[i] * old_pos[col];
                i++;
            }
        }
    }

    var x = point_4d.x;
    var y = point_4d.y;
    var z = point_4d.z;
    var w = point_4d.w;

    // Apply 4D perspective transformation.
    let w_divisor = 1.0 + w * projection_params.w_factor_4d;
    x = x / w_divisor;
    y = y / w_divisor;
    z = z / w_divisor;

    // Apply 3D perspective transformation.
    let z_divisor = 1.0 + (projection_params.fov_signum - z) * projection_params.w_factor_3d;
    w = z_divisor;

    vertex_3d_position_array[index] = vec4(x, y, z, w);
}
