//! Color constants.

const ALPHA: f32 = 0.75;

const BACKGROUND: [f32; 3] = [0.3, 0.3, 0.3];

pub const OUTLINE_BLACK: [f32; 4] = [0.0, 0.0, 0.0, 1.0];
pub const OUTLINE_WHITE: [f32; 4] = [1.0, 1.0, 1.0, 1.0];

pub const RED: [f32; 3] = [0.75, 0.0, 0.05];
pub const ORANGE: [f32; 3] = [0.75, 0.3, 0.0];
pub const WHITE: [f32; 3] = [0.75, 0.75, 0.75];
pub const YELLOW: [f32; 3] = [0.65, 0.75, 0.2];
pub const GREEN: [f32; 3] = [0.0, 0.6, 0.0];
pub const BLUE: [f32; 3] = [0.0, 0.3, 0.75];
pub const PURPLE: [f32; 3] = [0.5, 0.2, 0.75];
pub const PINK: [f32; 3] = [0.75, 0.45, 0.6];

/// Returns the background color.
pub fn get_bg() -> (f32, f32, f32, f32) {
    let [r, g, b] = BACKGROUND;
    (r, g, b, 1.0)
}
