use serde::{ser::SerializeMap, Deserialize, Serialize};
use std::collections::HashMap;

use super::PerPuzzle;
use crate::puzzle::{traits::*, Face, PuzzleTypeEnum};
use crate::serde_impl::hex_color;

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
#[serde(default)]
pub struct ColorPreferences {
    pub default_opacity: f32,
    pub hidden_opacity: f32,
    pub hovered_opacity: f32,

    #[serde(with = "hex_color")]
    pub background: egui::Color32,
    #[serde(with = "hex_color")]
    pub blind_face: egui::Color32,
    pub blindfold: bool,

    pub faces: PerPuzzle<HashMap<String, FaceColor>>,
}
impl std::ops::Index<(PuzzleTypeEnum, Face)> for ColorPreferences {
    type Output = egui::Color32;

    fn index(&self, (puzzle_type, face): (PuzzleTypeEnum, Face)) -> &Self::Output {
        self.faces
            .get(puzzle_type)
            .and_then(|face_colors| face_colors.get(puzzle_type.info(face).symbol))
            .map(|color| &color.0)
            .unwrap_or(&self.blind_face)
    }
}
impl std::ops::IndexMut<(PuzzleTypeEnum, Face)> for ColorPreferences {
    fn index_mut(&mut self, (puzzle_type, face): (PuzzleTypeEnum, Face)) -> &mut Self::Output {
        &mut self
            .faces
            .entry(puzzle_type)
            .or_default()
            .entry(puzzle_type.info(face).symbol.to_owned())
            .or_insert(FaceColor(self.blind_face))
            .0
    }
}

// TODO: rename this type and use it for all colors. also impl display
#[derive(Serialize, Deserialize, Debug, Default, Clone)]
#[serde(transparent)]
pub struct FaceColor(#[serde(with = "hex_color")] pub egui::Color32);

impl ColorPreferences {
    pub fn face_colors_list(&self, ty: PuzzleTypeEnum) -> Vec<egui::Color32> {
        let faces = &self.faces[ty];
        ty.faces()
            .iter()
            .map(|face| match faces.get(face.symbol) {
                Some(c) => c.0,
                None => self.blind_face,
            })
            .collect()
    }
}
