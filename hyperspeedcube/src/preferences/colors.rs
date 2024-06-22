use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
#[serde(default)]
pub struct SavedColor {
    pub name: String,
    pub rgb: [u8; 3],
}
