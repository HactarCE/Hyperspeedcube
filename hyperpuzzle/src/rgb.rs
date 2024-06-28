use std::fmt;
use std::str::FromStr;

use serde::de::Error;

/// 8-bit sRGB color that serializes to a string like `"#ff00ff"`.
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, Hash)]
pub struct Rgb {
    /// sRGB component values.
    pub rgb: [u8; 3],
}
impl fmt::Display for Rgb {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", color_to_hex_string(self.rgb))
    }
}
impl FromStr for Rgb {
    type Err = hex::FromHexError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let rgb = color_from_hex_str(s)?;
        Ok(Rgb { rgb })
    }
}
impl serde::Serialize for Rgb {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.to_string().serialize(serializer)
    }
}
impl<'de> serde::Deserialize<'de> for Rgb {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        s.parse::<Self>().map_err(D::Error::custom)
    }
}

/// Serializes a color to a hex string like `#ff00ff`.
fn color_to_hex_string(rgb: [u8; 3]) -> String {
    format!("#{}", hex::encode(rgb))
}

/// Deserializes a color from a hex string like `#ff00ff` or `#f0f`.
fn color_from_hex_str(s: &str) -> Result<[u8; 3], hex::FromHexError> {
    let mut rgb = [0_u8; 3];
    let s = s.strip_prefix('#').unwrap_or(s).trim();
    match s.len() {
        3 => {
            let s = &s.chars().flat_map(|c| [c, c]).collect::<String>();
            hex::decode_to_slice(&s, &mut rgb)?;
        }
        _ => hex::decode_to_slice(s, &mut rgb)?,
    }
    Ok(rgb)
}
