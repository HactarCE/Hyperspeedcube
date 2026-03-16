//! Serialization/deserialization of [`BitVec`] as a string using a base-10
//! length, then a `:`, then a sequence of hexadecimal digits.
//!
//! Only [`bitvec::order::Lsb0`] (the default in the [`bitvec`] crate) is
//! supported.

use bitvec::vec::BitVec;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Serializes a [`BitVec`] using [`bitvec_to_b16_string()`].
pub fn serialize<S: Serializer>(value: &BitVec, serializer: S) -> Result<S::Ok, S::Error> {
    bitvec_to_b16_string(value).serialize(serializer)
}

/// Deserializes a [`BitVec`] using [`b16_string_to_bitvec()`].
pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<BitVec, D::Error> {
    b16_string_to_bitvec(&<String>::deserialize(deserializer)?)
        .ok_or_else(|| serde::de::Error::custom("invalid bitstring"))
}

/// Serializes a [`BitVec`] as a base-10 length, then a `:`, then a sequence of
/// hexadecimal digits. The least significant hex digits are first.
pub fn bitvec_to_b16_string(bits: &BitVec) -> String {
    let mut ret = bits.len().to_string();
    ret.push(':');
    for chunk in bits.chunks(4) {
        let nibble = (0..4)
            .map(|i| match chunk.get(i) {
                Some(bit) => (*bit as u32) << i,
                None => 0,
            })
            .sum();
        ret.push(char::from_digit(nibble, 16).unwrap_or('?'));
    }
    ret
}

/// Deserializes a [`BitVec`] from a string containing a base-10 length, then a
/// `:`, then a sequence of hexadecimal digits. The least significant hex digits
/// must be first. Extra bits are truncated, and missing bits are assumed to be
/// 0.
///
/// Returns `None` if the string is malformed.
pub fn b16_string_to_bitvec(string: &str) -> Option<BitVec> {
    let (bit_count_str, contents) = string.split_once(':')?;
    let bit_count: usize = bit_count_str.parse().ok()?;
    if !contents.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    Some(
        contents
            .chars()
            .flat_map(|c| {
                let nibble = c.to_digit(16).unwrap_or(0); // should always succeed
                [
                    nibble & 1 != 0,
                    nibble & 2 != 0,
                    nibble & 4 != 0,
                    nibble & 8 != 0,
                ]
            })
            .chain(std::iter::repeat(false))
            .take(bit_count)
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_b16_encode_decode() {
        for s in [
            "79:f4add8920abe83143362",
            "0:", // empty
        ] {
            assert_eq!(s, bitvec_to_b16_string(&b16_string_to_bitvec(s).unwrap()));
        }

        // uppercase is ok
        assert_eq!(
            "79:f4add8920abe83143362",
            bitvec_to_b16_string(&b16_string_to_bitvec("79:F4ADD8920ABE83143362").unwrap()),
        );

        // extra bits are truncated
        assert_eq!(
            "7:f7",
            bitvec_to_b16_string(&b16_string_to_bitvec("7:ffff").unwrap()),
        );

        // missing bits are assumed 0
        assert_eq!(
            "7:f0",
            bitvec_to_b16_string(&b16_string_to_bitvec("7:f").unwrap()),
        );

        for s in [
            "f4ad",  // missing length
            ":f4ad", // empty length
            "4:g",   // invalid digit
        ] {
            assert_eq!(None, b16_string_to_bitvec(s));
        }
    }
}
