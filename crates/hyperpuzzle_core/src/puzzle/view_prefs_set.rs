use serde::{Deserialize, Serialize};
use strum::{AsRefStr, Display};

/// Which set of view settings to use for the puzzle UI.
#[derive(Serialize, Deserialize, Debug, Display, AsRefStr, Copy, Clone, PartialEq, Eq, Hash)]
pub enum PuzzleViewPreferencesSet {
    /// Perspective 3D rendering.
    #[serde(rename = "perspective_3d")]
    #[strum(serialize = "3D")]
    Dim3D,
    /// Perspective 4D rendering.
    #[serde(rename = "perspective_4d")]
    #[strum(serialize = "4D+")]
    Dim4D,
}
impl PuzzleViewPreferencesSet {
    /// Returns the puzzle view preferences set for a perspective-rendered
    /// Euclidean puzzle based on its number of dimensions.
    pub fn from_ndim(ndim: u8) -> Self {
        match ndim {
            ..=3 => Self::Dim3D,
            4.. => Self::Dim4D,
        }
    }
}
