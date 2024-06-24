use core::fmt;
use std::{collections::HashMap, str::FromStr};

use serde::{de::Error, Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
#[serde(default)]
pub struct SavedCustomColor {
    pub name: String,
    pub rgb: Rgb,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
#[serde(default)]
pub struct SavedCustomColorSet {
    pub name: String,
    pub colors: Vec<Rgb>,
}

#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, Hash)]
pub struct Rgb {
    pub rgb: [u8; 3],
}
impl fmt::Display for Rgb {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", crate::util::color_to_hex_string(self.rgb))
    }
}
impl FromStr for Rgb {
    type Err = hex::FromHexError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let rgb = crate::util::color_from_hex_str(s)?;
        Ok(Rgb { rgb })
    }
}
impl serde::Serialize for Rgb {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.to_string().serialize(serializer)
    }
}
impl<'de> serde::Deserialize<'de> for Rgb {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        s.parse::<Self>().map_err(D::Error::custom)
    }
}
impl Rgb {
    pub fn lerp(a: Self, b: Self, t: f32) -> Self {
        let a = a.to_egui_rgba();
        let b = b.to_egui_rgba();
        let [r, g, b, _a] = hypermath::util::lerp(a, b, t).to_srgba_unmultiplied();
        Self { rgb: [r, g, b] }
    }
    fn to_egui_rgba(self) -> egui::Rgba {
        let [r, g, b] = self.rgb;
        egui::Rgba::from_srgba_unmultiplied(r, g, b, 255)
    }
}
