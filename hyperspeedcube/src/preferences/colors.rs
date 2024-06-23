use serde::{Deserialize, Serialize};

use crate::serde_impl::{hex_color, hex_color_list};

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
#[serde(default)]
pub struct SavedColor {
    pub name: String,
    #[serde(with = "hex_color")]
    pub rgb: [u8; 3],
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
#[serde(default)]
pub struct SavedColorSet {
    pub name: String,
    #[serde(with = "hex_color_list")]
    pub colors: Vec<[u8; 3]>,
}
