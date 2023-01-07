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
            .unwrap_or_else(|| default_colors(facet))
    }
}
impl<'a> IndexMut<(&'a PuzzleType, Facet)> for ColorPreferences {
    fn index_mut(&mut self, (puzzle_type, facet): (&'a PuzzleType, Facet)) -> &mut Self::Output {
        &mut self
            .facets
            .entry(puzzle_type)
            .or_default()
            .entry(puzzle_type.info(facet).name.clone())
            .or_insert_with(|| FacetColor(*default_colors(facet)))
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

// todo: make this not a hack
fn default_colors(facet: Facet) -> &'static egui::Color32 {
    const COLORS: [egui::Color32; 16] = [
        egui::Color32::from_rgb(255, 0, 0),        // Red
        egui::Color32::from_rgb(255, 0xaa, 00),    // Orange
        egui::Color32::from_rgb(0, 255, 0),        // Green
        egui::Color32::from_rgb(0, 0, 255),        // Blue
        egui::Color32::from_rgb(255, 255, 255),    // White
        egui::Color32::from_rgb(255, 255, 0),      // Yellow
        egui::Color32::from_rgb(255, 100, 255),    // Pink
        egui::Color32::from_rgb(0x50, 0, 255),     // Purple
        egui::Color32::from_rgb(100, 0, 70),       // Plum
        egui::Color32::from_rgb(0x9d, 0x60, 255),  // Lilac
        egui::Color32::from_rgb(0, 255, 255),      // Cyan
        egui::Color32::from_rgb(0, 100, 100),      // Teal
        egui::Color32::from_rgb(0x22, 0x22, 0x22), // Dark Grey
        egui::Color32::from_rgb(0x88, 0x88, 0x88), // Light Grey
        egui::Color32::from_rgb(0, 100, 0),        // Dark Green
        egui::Color32::from_rgb(0, 0, 100),        // Dark Blue
    ];
    const FALLBACK_COLOR: egui::Color32 = egui::Color32::from_rgb(0x66, 0x66, 0x66);
    COLORS.get(facet.0 as usize).unwrap_or(&FALLBACK_COLOR)
}
