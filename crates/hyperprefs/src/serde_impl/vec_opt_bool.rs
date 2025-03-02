use hypermath::IndexNewtype;
use hypermath::collections::GenericVec;
use serde::{Deserializer, Serializer};

pub fn serialize<S: Serializer, I: IndexNewtype>(
    value: &GenericVec<I, Option<bool>>,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    super::hex_bitvec::serialize(
        &value
            .iter_values()
            .flat_map(|&v| [v.is_some(), v == Some(true)])
            .collect(),
        serializer,
    )
}

pub fn deserialize<'de, D: Deserializer<'de>, I: IndexNewtype>(
    deserializer: D,
) -> Result<GenericVec<I, Option<bool>>, D::Error> {
    Ok(super::hex_bitvec::deserialize(deserializer)?
        .chunks_exact(2)
        .map(|pair| pair[0].then_some(pair[1]))
        .collect())
}
