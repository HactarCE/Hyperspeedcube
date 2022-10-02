use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::ops::{Index, IndexMut};

use super::PerPuzzleFamily;
use crate::puzzle::{traits::*, Facet, PuzzleType};
use crate::serde_impl::hex_color;

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
#[serde(default)]
pub struct ColorPreferences {
    #[serde(with = "hex_color")]
    pub background: egui::Color32,
    #[serde(with = "hex_color", alias = "blind_face")]
    pub blind_sticker: egui::Color32,
    pub blindfold: bool,

    pub facets: PerPuzzleFamily<BTreeMap<String, FacetColor>>,
}
impl<'a> Index<(&'a PuzzleType, Facet)> for ColorPreferences {
    type Output = egui::Color32;

    fn index(&self, (puzzle_type, facet): (&'a PuzzleType, Facet)) -> &Self::Output {
        self.facets
            .get(puzzle_type)
            .and_then(|facet_colors| facet_colors.get(&puzzle_type.info(facet).name))
            .map(|color| &color.0)
            .unwrap_or(&self.blind_sticker)
    }
}
impl<'a> IndexMut<(&'a PuzzleType, Facet)> for ColorPreferences {
    fn index_mut(&mut self, (puzzle_type, facet): (&'a PuzzleType, Facet)) -> &mut Self::Output {
        &mut self
            .facets
            .entry(puzzle_type)
            .or_default()
            .entry(puzzle_type.info(facet).name.clone())
            .or_insert(FacetColor(self.blind_sticker))
            .0
    }
}

// TODO: rename this type and use it for all colors. also impl display
#[derive(Serialize, Deserialize, Debug, Default, Clone)]
#[serde(transparent)]
pub struct FacetColor(#[serde(with = "hex_color")] pub egui::Color32);

impl ColorPreferences {
    pub fn facet_colors_list(&self, ty: &PuzzleType) -> Vec<egui::Color32> {
        let facets = &self.facets[ty];
        ty.shape
            .facets
            .iter()
            .map(|facet| match facets.get(&facet.name) {
                Some(c) => c.0,
                None => self.blind_sticker,
            })
            .collect()
    }
}
