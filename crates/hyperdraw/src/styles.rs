use hyperprefs::StyleColorMode;

/// Values for how to draw a piece, depending on its style state.
#[derive(Debug, Default, Copy, Clone, PartialEq)]
pub struct PieceStyleValues {
    /// Sticker face opacity.
    pub face_opacity: u8, // TODO: linear or gamma??
    /// Sticker face color.
    pub face_color: StyleColorMode,

    /// Sticker outline opacity.
    pub outline_opacity: u8,
    /// Sticker outline color.
    pub outline_color: StyleColorMode,
    /// Whether to darken the sticker outline color based on the angle to the
    /// light source.
    pub outline_lighting: bool,

    /// Thickness of the outline, in approximately pixel-sized units.
    pub outline_size: f32,
}
