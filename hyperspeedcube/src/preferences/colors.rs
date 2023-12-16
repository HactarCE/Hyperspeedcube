use serde::{Deserialize, Serialize};

use crate::serde_impl::hex_color;

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
#[serde(default)]
pub struct ColorPreferences {
    #[serde(with = "hex_color")]
    pub background: egui::Color32,
    #[serde(with = "hex_color")]
    pub blind_face: egui::Color32,
    pub blindfold: bool,
}
