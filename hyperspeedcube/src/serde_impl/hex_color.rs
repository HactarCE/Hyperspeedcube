// TODO: make a custom type

use serde::de::Error;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

pub fn serialize<S: Serializer>(rgb: &egui::Color32, serializer: S) -> Result<S::Ok, S::Error> {
    to_str(rgb).serialize(serializer)
}

pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<egui::Color32, D::Error> {
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
