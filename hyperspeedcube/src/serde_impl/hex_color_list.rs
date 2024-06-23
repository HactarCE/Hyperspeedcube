// TODO: make a custom type

use itertools::Itertools;
use serde::de::Error;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

pub fn serialize<S: Serializer>(rgb: &Vec<[u8; 3]>, serializer: S) -> Result<S::Ok, S::Error> {
    rgb.iter()
        .map(super::hex_color::to_str)
        .collect_vec()
        .serialize(serializer)
}

pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Vec<[u8; 3]>, D::Error> {
    <Vec<String>>::deserialize(deserializer)?
        .into_iter()
        .map(|s| super::hex_color::from_str(&s))
        .try_collect()
        .map_err(D::Error::custom)
}
