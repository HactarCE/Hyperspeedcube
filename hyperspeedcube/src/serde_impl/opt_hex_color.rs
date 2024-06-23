// TODO: make a custom type

use serde::de::Error;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

pub fn serialize<S: Serializer>(rgb: &Option<[u8; 3]>, serializer: S) -> Result<S::Ok, S::Error> {
    rgb.as_ref()
        .map(super::hex_color::to_str)
        .serialize(serializer)
}

pub fn deserialize<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<[u8; 3]>, D::Error> {
    <Option<String>>::deserialize(deserializer)?
        .as_deref()
        .map(super::hex_color::from_str)
        .transpose()
        .map_err(D::Error::custom)
}
