// TODO: make a custom type

use serde::de::Error;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

pub fn serialize<S: Serializer>(rgb: &[u8; 3], serializer: S) -> Result<S::Ok, S::Error> {
    to_str(rgb).serialize(serializer)
}

pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<[u8; 3], D::Error> {
    from_str(&String::deserialize(deserializer)?).map_err(D::Error::custom)
}

pub fn to_str(rgb: &[u8; 3]) -> String {
    format!("#{}", hex::encode(rgb))
}

pub fn from_str(s: &str) -> Result<[u8; 3], hex::FromHexError> {
    let mut rgb = [0_u8; 3];
    let s = s
        .chars()
        .filter(|c| matches!(c, '0'..='9' | 'a'..='f' | 'A'..='F'))
        .collect::<String>();
    hex::decode_to_slice(&s, &mut rgb).map(|()| rgb)
}
