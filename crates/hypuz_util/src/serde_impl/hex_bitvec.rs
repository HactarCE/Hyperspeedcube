//! Serialization/deserialization of [`BitVec`] as a string using a base-10
//! length, then a `:`, then a sequence of hexadecimal digits.
//!
//! Only [`bitvec::order::Lsb0`] (the default in the [`bitvec`] crate) is
//! supported.

use bitvec::vec::BitVec;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Serializes a [`BitVec`] using [`crate::bitvec_to_b16_string()`].
pub fn serialize<S: Serializer>(value: &BitVec, serializer: S) -> Result<S::Ok, S::Error> {
    crate::bitvec_to_b16_string(value).serialize(serializer)
}

/// Deserializes a [`BitVec`] using [`crate::b16_string_to_bitvec()`].
pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<BitVec, D::Error> {
    crate::b16_string_to_bitvec(&<String>::deserialize(deserializer)?)
        .ok_or_else(|| serde::de::Error::custom("invalid bitstring"))
}
