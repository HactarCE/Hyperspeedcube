/// Returns evenly-spaced layer depths, including both endpoints
///
/// Typically `start > stop`
fn inclusive(start, stop, layer_count) {
    assert_eq(type_of(layer_count), "i64");

    if layer_count < 1 { return []; }

    let ret = [];
    for i in 0..layer_count.to_int() + 1 {
        ret.push(start + (stop - start) * i / layer_count);
    }
    ret
}

/// Returns evenly-spaced layer depths for half of a puzzle
///
/// For even numbers of layers, includes both endpoints
/// For odd numbers of layers, includes `start` but not `stop`
///
/// Expects `start > stop`
fn even_odd(start, stop, layer_count) {
    assert_eq(type_of(layer_count), "i64");

    let half_layer_size = (stop - start) / layer_count;
    if layer_count.is_odd() {
        stop -= half_layer_size;
    }

    inclusive(start, stop, floor(layer_count/2))
}

/// Returns evenly-spaced layer depths, including both endpoints and with `INF`
/// and `-INF` on either side.
fn inclusive_inf(start, stop, layer_count) {
    assert_eq(type_of(layer_count), "i64");

    if layer_count < 1 {
        []
    } else if layer_count == 1 {
        [INF, -INF]
    } else if layer_count == 2 {
        [INF, (start+stop)/2, -INF]
    } else {
        [INF] + inclusive(start, stop, layer_count-2) + [-INF]
    }
}

/// Returns evenly-spaced layer depths, excluding both endpoints and with `INF`
/// on one side.
fn exclusive_centered(center, half_range, cut_count) {
    assert_eq(type_of(cut_count), "i64");

    if cut_count == 0 {
        []
    } else if cut_count == 1 {
        [center]
    } else {
        let half_layer_height = half_range / (cut_count + 1);
        let outermost_cut = center + half_range - half_layer_height;
        let innermost_cut = center - half_range + half_layer_height;
        inclusive(outermost_cut, innermost_cut, cut_count-1)
    }
}
