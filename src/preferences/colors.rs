use serde::{Deserialize, Serialize};

use super::{DeserializePerPuzzle, PerPuzzle};
use crate::colors;
use crate::puzzle::{PuzzleType, PuzzleTypeTrait};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ColorPreferences {
    pub sticker_opacity: f32,
    pub outline_opacity: f32,

    pub faces: PerPuzzle<FaceColors>,

    pub background: [f32; 3],
    pub outline: [f32; 3],

    pub label_fg: [f32; 4],
    pub label_bg: [f32; 4],
}
impl Default for ColorPreferences {
    fn default() -> Self {
        Self {
            sticker_opacity: 1.0,
            outline_opacity: 1.0,

            faces: PerPuzzle::default(),

            background: colors::DEFAULT_BACKGROUND,
            outline: colors::DEFAULT_OUTLINE,

            label_fg: colors::DEFAULT_LABEL_FG,
            label_bg: colors::DEFAULT_LABEL_BG,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct FaceColors(pub Vec<[f32; 3]>);
impl std::ops::Index<usize> for FaceColors {
    type Output = [f32; 3];

    fn index(&self, index: usize) -> &Self::Output {
        &self.0[index]
    }
}
impl std::ops::IndexMut<usize> for FaceColors {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.0[index]
    }
}
impl DeserializePerPuzzle<'_> for FaceColors {
    type Proxy = Self;

    fn deserialize_from(mut face_colors: Self, ty: PuzzleType) -> Self {
        face_colors.0.resize(ty.faces().len(), colors::GRAY);
        face_colors
    }
}
