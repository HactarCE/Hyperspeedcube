//! Color constants.

const ALPHA: f32 = 0.75;

const BACKGROUND: [f32; 3] = [0.3, 0.3, 0.3];

pub const OUTLINE_BLACK: [f32; 4] = [0.0, 0.0, 0.0, 1.0];
pub const OUTLINE_WHITE: [f32; 4] = [1.0, 1.0, 1.0, 1.0];

const RED: [f32; 3] = [0.8, 0.0, 0.0];
const ORANGE: [f32; 3] = [0.6, 0.2, 0.0];
const WHITE: [f32; 3] = [0.8, 0.8, 0.8];
const YELLOW: [f32; 3] = [0.8, 0.8, 0.0];
const GREEN: [f32; 3] = [0.0, 0.5, 0.0];
const BLUE: [f32; 3] = [0.0, 0.1, 0.6];

/// Returns the background color.
pub fn get_bg() -> (f32, f32, f32, f32) {
    let [r, g, b] = BACKGROUND;
    (r, g, b, 1.0)
}

/// Returns the color for the face with the given index.
pub fn get_color(i: usize) -> [f32; 4] {
    let [r, g, b] = match i {
        0 => RED,
        1 => ORANGE,
        2 => WHITE,
        3 => YELLOW,
        4 => GREEN,
        5 => BLUE,
        _ => panic!("Invalid color index"),
    };
    [r, g, b, ALPHA]
}
