use strum::{AsRefStr, Display};

/// Which set of view settings to use for the puzzle UI.
#[derive(Debug, Display, AsRefStr, Copy, Clone, PartialEq, Eq, Hash)]
pub enum PuzzleViewPreferencesSet {
    /// Perspective rendering in Euclidean space.
    #[strum(serialize = "{0}")]
    Perspective(PerspectiveDim),
}

/// Perspective rendering dimension.
#[derive(Debug, Display, AsRefStr, Copy, Clone, PartialEq, Eq, Hash)]
pub enum PerspectiveDim {
    /// Perspective 3D rendering.
    #[strum(serialize = "3D")]
    Dim3D,
    /// Perspective 4D rendering.
    #[strum(serialize = "4D+")]
    Dim4D,
}
impl PerspectiveDim {
    /// Returns the puzzle view preferences set for a perspective-rendered
    /// Euclidean puzzle based on its number of dimensions.
    pub fn from_ndim(ndim: u8) -> Self {
        match ndim {
            ..=3 => Self::Dim3D,
            4.. => Self::Dim4D,
        }
    }
}
