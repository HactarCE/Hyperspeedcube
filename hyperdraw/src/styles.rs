use hyperprefs::StyleColorMode;

/// Values for how to draw a piece, depending on its style state.
#[derive(Debug, Default, Copy, Clone, PartialEq)]
pub struct PieceStyleValues {
    pub face_opacity: u8, // TODO: linear or gamma??
    pub face_color: StyleColorMode,

    pub outline_opacity: u8,
    pub outline_color: StyleColorMode,
    pub outline_lighting: bool,

    pub outline_size: f32,
}
