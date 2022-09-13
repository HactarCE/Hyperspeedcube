use bitvec::vec::BitVec;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

pub fn serialize<S>(value: &BitVec, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    bitvec_to_b16_string(value).serialize(serializer)
}

pub fn deserialize<'de, D>(deserializer: D) -> Result<BitVec, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(b16_string_to_bitvec(&<String>::deserialize(deserializer)?))
}

pub mod opt {
    use super::*;

    pub fn serialize<S>(value: &Option<BitVec>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        value
            .as_ref()
            .map(bitvec_to_b16_string)
            .serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<BitVec>, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(<Option<String>>::deserialize(deserializer)?.map(|s| b16_string_to_bitvec(&s)))
    }
}

pub fn bitvec_to_b16_string(bits: &BitVec) -> String {
    bits.chunks(4)
        .into_iter()
        .map(|chunk| {
            let nibble = (0..4)
                .map(|i| match chunk.get(i) {
                    Some(bit) => (*bit as u32) << i,
                    None => 0,
                })
                .sum();
            char::from_digit(nibble, 16).unwrap_or('?')
        })
        .collect()
}

pub fn b16_string_to_bitvec(string: &str) -> BitVec {
    string
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
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_b16_encode_decode() {
        let s = "f4add8920abe83143362";
        assert_eq!(s, bitvec_to_b16_string(&b16_string_to_bitvec(s)));
    }
}
