use std::fmt;
use std::str::FromStr;

#[cfg(feature = "oklab")]
pub use oklab::Oklab;
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
impl Rgb {
    /// Pure black
    pub const BLACK: Rgb = Rgb { rgb: [0; 3] };
    /// Pure white
    pub const WHITE: Rgb = Rgb { rgb: [255; 3] };
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
            hex::decode_to_slice(s, &mut rgb)?;
        }
        _ => hex::decode_to_slice(s, &mut rgb)?,
    }
    Ok(rgb)
}

#[cfg(feature = "ecolor")]
mod ecolor_convert {
    use super::*;

    impl From<Rgb> for ecolor::Color32 {
        fn from(value: Rgb) -> Self {
            let [r, g, b] = value.rgb;
            ecolor::Color32::from_rgb(r, g, b)
        }
    }
    impl From<ecolor::Color32> for Rgb {
        fn from(value: ecolor::Color32) -> Self {
            let [r, g, b, _] = value.to_array();
            Rgb { rgb: [r, g, b] }
        }
    }

    impl From<Rgb> for ecolor::Rgba {
        fn from(value: Rgb) -> Self {
            let [r, g, b] = value.rgb;
            ecolor::Rgba::from_srgba_unmultiplied(r, g, b, 255)
        }
    }
    impl From<ecolor::Rgba> for Rgb {
        fn from(value: ecolor::Rgba) -> Self {
            let [r, g, b, _] = value.to_srgba_unmultiplied();
            Rgb { rgb: [r, g, b] }
        }
    }

    impl Rgb {
        /// Converts an [`ecolor::Color32`] to an [`Rgb`].
        pub fn to_egui_color32(self) -> ecolor::Color32 {
            self.into()
        }
        /// Converts an [`Rgb`] to an [`ecolor::Color32`].
        pub fn from_egui_color32(color: ecolor::Color32) -> Self {
            color.into()
        }

        /// Converts an [`ecolor::Rgba`] to an [`Rgb`].
        pub fn to_egui_rgba(self) -> ecolor::Rgba {
            self.into()
        }
        /// Converts an [`Rgb`] to an [`ecolor::Rgba`].
        pub fn from_egui_rgba(color: ecolor::Rgba) -> Self {
            color.into()
        }

        /// Interpolates between two colors in linear color space.
        pub fn mix(a: Self, b: Self, t: f32) -> Self {
            hypermath::util::lerp(a.to_egui_rgba(), b.to_egui_rgba(), t).into()
        }
    }
}

#[cfg(feature = "oklab")]
mod oklab_convert {
    use super::*;

    impl From<Rgb> for Oklab {
        fn from(value: Rgb) -> Self {
            let [r, g, b] = value.rgb;
            oklab::srgb_to_oklab(oklab::Rgb { r, g, b })
        }
    }
    impl From<Oklab> for Rgb {
        fn from(value: Oklab) -> Self {
            let oklab::Rgb { r, g, b } = oklab::oklab_to_srgb(value);
            Rgb { rgb: [r, g, b] }
        }
    }

    impl Rgb {
        /// Converts an [`oklab::Oklab`] to an [`Rgb`].
        pub fn to_oklab(self) -> oklab::Oklab {
            self.into()
        }
        /// Converts an [`Rgb`] to an [`oklab::Oklab`].
        pub fn from_oklab(color: oklab::Oklab) -> Self {
            color.into()
        }
    }
}
