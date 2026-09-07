use std::str::FromStr;

use hyperkdl::ValueSchemaProxy;
use hyperpuzzle_core::{LayerMask, ScrambleType, Timestamp};
use hypuz_notation::Layer;
use kdl::*;

/// KDL serialization proxy type for some types defined in `hyperpuzzle_core`.
pub struct KdlProxy;

impl ValueSchemaProxy<LayerMask> for KdlProxy {
    fn proxy_from_kdl_value(value: &KdlValue) -> Option<LayerMask> {
        if let Some(bits) = value.as_integer() {
            // for compatibility with schema v2
            Some(LayerMask::from_iter(
                (0..u32::BITS as usize)
                    .filter(|&i| bits & (1 << i) != 0)
                    .filter_map(Layer::from_index),
            ))
        } else if let Some(s) = value.as_string() {
            LayerMask::from_hex_str(s)
        } else {
            None
        }
    }
    fn proxy_to_kdl_value(value: &LayerMask) -> KdlValue {
        KdlValue::String(value.to_hex_string())
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
