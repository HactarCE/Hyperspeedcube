use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::ops::{Index, IndexMut};

use super::PerPuzzleFamily;
use crate::puzzle::{traits::*, Face, PuzzleType};
use crate::serde_impl::hex_color;

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
#[serde(default)]
pub struct ColorPreferences {
    #[serde(with = "hex_color")]
    pub background: egui::Color32,
    #[serde(with = "hex_color")]
    pub blind_face: egui::Color32,
    pub blindfold: bool,

    pub faces: PerPuzzleFamily<BTreeMap<String, FaceColor>>,
}
impl<'a> Index<(&'a PuzzleType, Face)> for ColorPreferences {
    type Output = egui::Color32;

    fn index(&self, (puzzle_type, face): (&'a PuzzleType, Face)) -> &Self::Output {
        self.faces
            .get(puzzle_type)
            .and_then(|face_colors| face_colors.get(&puzzle_type.info(face).name))
            .map(|color| &color.0)
            .unwrap_or(&self.blind_face)
    }
}
impl<'a> IndexMut<(&'a PuzzleType, Face)> for ColorPreferences {
    fn index_mut(&mut self, (puzzle_type, face): (&'a PuzzleType, Face)) -> &mut Self::Output {
        &mut self
            .faces
            .entry(puzzle_type)
            .or_default()
            .entry(puzzle_type.info(face).name.clone())
            .or_insert(FaceColor(self.blind_face))
            .0
    }
}

// TODO: rename this type and use it for all colors. also impl display
#[derive(Serialize, Deserialize, Debug, Default, Clone)]
#[serde(transparent)]
pub struct FaceColor(#[serde(with = "hex_color")] pub egui::Color32);

impl ColorPreferences {
    pub fn face_colors_list(&self, ty: &PuzzleType) -> Vec<egui::Color32> {
        let faces = &self.faces[ty];
        ty.shape
            .faces
            .iter()
            .map(|face| match faces.get(&face.name) {
                Some(c) => c.0,
                None => self.blind_face,
            })
            .collect()
    }
}
