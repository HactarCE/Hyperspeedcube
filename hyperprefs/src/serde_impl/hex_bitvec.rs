use bitvec::vec::BitVec;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

pub fn serialize<S: Serializer>(value: &BitVec, serializer: S) -> Result<S::Ok, S::Error> {
    bitvec_to_b16_string(value).serialize(serializer)
}

pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<BitVec, D::Error> {
    Ok(b16_string_to_bitvec(&<String>::deserialize(deserializer)?))
}

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

pub fn b16_string_to_bitvec(string: &str) -> BitVec {
    let (len, contents) = string.split_once(':').unwrap_or(("", string));
    contents
        .chars()
        .flat_map(|c| {
            let nibble = c.to_digit(16).unwrap_or(0);
            [
                nibble & 1 != 0,
                nibble & 2 != 0,
                nibble & 4 != 0,
                nibble & 8 != 0,
            ]
        })
        .take(len.parse().unwrap_or(string.len() * 4))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_b16_encode_decode() {
        let s = "79:f4add8920abe83143362";
        assert_eq!(s, bitvec_to_b16_string(&b16_string_to_bitvec(s)));
    }
}
