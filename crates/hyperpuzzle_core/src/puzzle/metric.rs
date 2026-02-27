use super::{LayeredTwist, Puzzle};

// TODO: move this function to hypuz_notation

/// Counts a sequence of twists using Slice Turn Metric.
pub fn count_stm(puzzle: &Puzzle, twists: impl IntoIterator<Item = LayeredTwist>) -> u64 {
    let mut count = 0;
    let mut prev_axis = None;
    let mut prev_layers = None;
    for twist in twists {
        let twist_info = &puzzle.twists.twists[twist.transform];
        let axis = twist_info.axis;

        if prev_axis == Some(axis) && prev_layers == Some(twist.layers) {
            // Same axis and layers as previous twist! This twist is free.
            continue;
        } else {
            prev_axis = Some(axis);
            prev_layers = Some(twist.layers);
            count += 1;
        };
    }
    count
}
