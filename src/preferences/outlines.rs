use crate::serde_impl::hex_color;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
#[serde(default)]
pub struct OutlinePreferences {
    pub default_size: f32,
    pub hidden_size: f32,
    pub hovered_size: f32,
    pub selected_size: f32,

    #[serde(with = "hex_color")]
    pub default_color: egui::Color32,
    #[serde(with = "hex_color")]
    pub hidden_color: egui::Color32,
    #[serde(with = "hex_color")]
    pub hovered_color: egui::Color32,
    #[serde(with = "hex_color")]
    pub selected_sticker_color: egui::Color32,
    #[serde(with = "hex_color")]
    pub selected_piece_color: egui::Color32,
}
