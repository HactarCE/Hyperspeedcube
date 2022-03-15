use serde::{ser::SerializeMap, Deserialize, Serialize};
use std::collections::HashMap;

use super::{DeserializePerPuzzle, PerPuzzle};
use crate::puzzle::{Face, PuzzleType, PuzzleTypeTrait};

mod hex_color {
    use serde::{de::Error, Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S: Serializer>(rgb: &egui::Color32, serializer: S) -> Result<S::Ok, S::Error> {
        to_str(rgb).serialize(serializer)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<egui::Color32, D::Error> {
        from_str(&String::deserialize(deserializer)?).map_err(D::Error::custom)
    }

    pub fn to_str(rgb: &egui::Color32) -> String {
        format!("#{}", hex::encode(&rgb.to_srgba_unmultiplied()[..3]))
    }

    pub fn from_str(s: &str) -> Result<egui::Color32, hex::FromHexError> {
        let mut ret = [0_u8; 3];
        let s = s
            .chars()
            .filter(|c| matches!(c, '0'..='9' | 'a'..='f' | 'A'..='F'))
            .collect::<String>();
        hex::decode_to_slice(&s, &mut ret).map(|()| {
            let [r, g, b] = ret;
            egui::Color32::from_rgb(r, g, b)
        })
    }
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
#[serde(default)]
pub struct ColorPreferences {
    pub sticker_opacity: f32,
    pub hidden_opacity: f32,

    pub faces: PerPuzzle<FaceColors>,

    #[serde(with = "hex_color")]
    pub blind_face: egui::Color32,
    #[serde(with = "hex_color")]
    pub background: egui::Color32,
    #[serde(with = "hex_color")]
    pub outline: egui::Color32,
}
impl std::ops::Index<Face> for ColorPreferences {
    type Output = egui::Color32;

    fn index(&self, index: Face) -> &Self::Output {
        &self.faces[index.ty()].colors[index.id()]
    }
}
impl std::ops::IndexMut<Face> for ColorPreferences {
    fn index_mut(&mut self, index: Face) -> &mut Self::Output {
        &mut self.faces[index.ty()].colors[index.id()]
    }
}

#[derive(Debug, Default, Clone)]
pub struct FaceColors {
    puzzle_type: PuzzleType,
    colors: Vec<egui::Color32>,
}
impl Serialize for FaceColors {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut map = serializer.serialize_map(Some(self.colors.len()))?;
        for (face, color) in self.puzzle_type.face_symbols().iter().zip(&self.colors) {
            map.serialize_entry(face, &hex_color::to_str(color))?;
        }
        map.end()
    }
}
impl std::ops::Index<usize> for FaceColors {
    type Output = egui::Color32;

    fn index(&self, index: usize) -> &Self::Output {
        &self.colors[index]
    }
}
impl std::ops::IndexMut<usize> for FaceColors {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.colors[index]
    }
}
impl DeserializePerPuzzle<'_> for FaceColors {
    type Proxy = HashMap<String, String>;

    fn deserialize_from(hex_codes: Self::Proxy, ty: PuzzleType) -> Self {
        Self {
            puzzle_type: ty,
            colors: ty
                .face_symbols()
                .iter()
                .map(|&face| {
                    hex_codes
                        .get(face)
                        .and_then(|hex_str| hex_color::from_str(hex_str).ok())
                        .unwrap_or(egui::Color32::GRAY)
                })
                .collect(),
        }
    }
}
