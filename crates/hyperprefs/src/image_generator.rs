use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Default, Clone, PartialEq)]
#[serde(default)]
pub struct ImageGeneratorPreferences {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dir: Option<PathBuf>,
    pub filename: String,

    pub width: u32,
    pub height: u32,
}
