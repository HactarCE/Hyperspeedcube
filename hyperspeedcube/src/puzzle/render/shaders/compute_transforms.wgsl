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
    ndim: u32,
}

let MAX_NDIM = 8;

@group(0) @binding(0) var<uniform> offset: u32;
@group(0) @binding(1) var<uniform> projection_params: ProjectionParams;

@group(1) @binding(0) var<storage, read> puzzle_transform: array<f32>;

@group(1) @binding(1) var<storage, read> piece_transform_array: array<f32>;

@group(1) @binding(2) var<storage, read> facet_shrink_center_array: array<f32>;

@group(1) @binding(3) var<storage, read> sticker_info_array: array<StickerInfo>;
@group(1) @binding(4) var<storage, read> sticker_shrink_center_array: array<f32>;

@group(1) @binding(5) var<storage, read> vertex_sticker_id_data: array<u32>;
@group(1) @binding(6) var<storage, read> vertex_position_array: array<f32>;
@group(1) @binding(7) var<storage, read_write> vertex_3d_position_array: array<vec4<f32>>;

@compute
@workgroup_size(64)
fn main(@builtin(global_invocation_id) global_invocation_id: vec3<u32>) {
    let total = arrayLength(&vertex_3d_position_array);
    let index = offset + global_invocation_id.x;
    if (index >= total) {
        return;
    }

    let ndim = projection_params.ndim;

    let sticker: u32 = vertex_sticker_id_data[index];
    let piece: u32 = sticker_info_array[sticker].piece;
    let facet: u32 = sticker_info_array[sticker].facet;

    // TODO: shrink stickers and facets

    var initial = array<f32, MAX_NDIM>();
    var vert_idx = ndim * index;
    for (var i = 0u; i < ndim; i++) {
        initial[i] = vertex_position_array[vert_idx];
        vert_idx++;
    }


    // Apply facet scaling.
    var j = facet * ndim;
    for (var i = 0u; i < ndim; i++) {
        initial[i] -= facet_shrink_center_array[j];
        initial[i] *= projection_params.facet_scale;
        initial[i] += facet_shrink_center_array[j];
        j++;
    }

    // Apply piece transformation.
    var old_pos = initial;
    var new_pos = array<f32, MAX_NDIM>();
    var i: u32 = ndim * ndim * piece;
    var base: u32 = ndim * index;
    for (var col = 0u; col < ndim; col++) {
        for (var row = 0u; row < ndim; row++) {
            new_pos[row] += piece_transform_array[i] * old_pos[col];
            i++;
        }
    }
    old_pos = new_pos;

    // Apply puzzle transformation and collapse to 4D.
    var point_4d = vec4<f32>();
    var i = 0;
    for (var col = 0u; col < ndim; col++) {
        // TODO: optimize this
        for (var row = 0u; row < ndim; row++) {
            if (row < 4u) {
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
