use std::str::FromStr;

use hyperkdl::ValueSchemaProxy;
use hyperpuzzle_core::{LayerMask, ScrambleType, Timestamp};
use kdl::*;

/// KDL serialization proxy type for some types defined in `hyperpuzzle_core`.
pub struct KdlProxy;

impl ValueSchemaProxy<LayerMask> for KdlProxy {
    fn proxy_from_kdl_value(value: &KdlValue) -> Option<LayerMask> {
        Some(LayerMask(u32::try_from(value.as_integer()?).ok()?))
    }
    fn proxy_to_kdl_value(value: &LayerMask) -> KdlValue {
        KdlValue::Integer(i128::from(value.0))
    }
}

impl ValueSchemaProxy<Timestamp> for KdlProxy {
    fn proxy_from_kdl_value(value: &KdlValue) -> Option<Timestamp> {
        Timestamp::from_str(value.as_string()?).ok()
    }
    fn proxy_to_kdl_value(value: &Timestamp) -> KdlValue {
        KdlValue::String(value.to_string())
    }
}

impl ValueSchemaProxy<ScrambleType> for KdlProxy {
    fn proxy_from_kdl_value(value: &KdlValue) -> Option<ScrambleType> {
        match value {
            KdlValue::String(s) => match s.as_str() {
                "full" => Some(ScrambleType::Full),
                _ => None,
            },
            KdlValue::Integer(n) => u32::try_from(*n).ok().map(ScrambleType::Partial),
            _ => None,
        }
    }
    fn proxy_to_kdl_value(value: &ScrambleType) -> KdlValue {
        match *value {
            ScrambleType::Full => KdlValue::from("full"),
            ScrambleType::Partial(n) => KdlValue::from(i128::from(n)),
        }
    }
}
